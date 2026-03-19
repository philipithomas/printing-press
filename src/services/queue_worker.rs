use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::models::email_suppression::EmailSuppression;
use crate::services::dns::MxValidator;
use crate::services::email_service::EmailService;

const MAX_ATTEMPTS: i32 = 5;

#[derive(Debug, sqlx::FromRow)]
struct QueuedEmail {
    id: i64,
    #[allow(dead_code)]
    subscriber_id: i64,
    #[allow(dead_code)]
    post_slug: String,
    newsletter: Option<String>,
    unsubscribe_token: Uuid,
    subject: Option<String>,
    html_content: Option<String>,
    preview_text: Option<String>,
    attempts: i32,
    email: String,
}

pub async fn run(
    pool: PgPool,
    email_service: EmailService,
    config: Config,
    mx_validator: MxValidator,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        if let Err(e) = process_batch(&pool, &email_service, &config, &mx_validator).await {
            tracing::error!("Queue worker error: {}", e);
        }
    }
}

async fn process_batch(
    pool: &PgPool,
    email_service: &EmailService,
    config: &Config,
    mx_validator: &MxValidator,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let batch = fetch_pending(pool, config.ses_rate_per_second as i64).await?;

    if batch.is_empty() {
        return Ok(());
    }

    tracing::debug!("Queue worker processing {} emails", batch.len());
    let delay = Duration::from_secs_f64(1.0 / config.ses_rate_per_second as f64);

    for queued in &batch {
        // Suppression check
        match EmailSuppression::is_suppressed(pool, &queued.email).await {
            Ok(true) => {
                tracing::info!(
                    "Queue item {} to {} suppressed (bounce/complaint)",
                    queued.id,
                    queued.email
                );
                mark_permanent_failure(
                    pool,
                    queued.id,
                    "Recipient is suppressed (bounce/complaint)",
                )
                .await?;
                continue;
            }
            Err(e) => {
                tracing::error!("Suppression check error for {}: {}", queued.email, e);
                // Fail open — don't block on suppression check errors
            }
            Ok(false) => {}
        }

        // MX validation
        match mx_validator.has_mx_records(&queued.email).await {
            Ok(false) => {
                tracing::info!(
                    "Queue item {} to {} has no MX records",
                    queued.id,
                    queued.email
                );
                mark_permanent_failure(pool, queued.id, "No MX records for recipient domain")
                    .await?;
                continue;
            }
            Err(e) => {
                tracing::warn!("DNS lookup error for {}: {}", queued.email, e);
                let backoff = next_attempt_delay(queued.attempts + 1);
                if queued.attempts + 1 >= MAX_ATTEMPTS {
                    mark_permanent_failure(
                        pool,
                        queued.id,
                        &format!("DNS lookup failed after retries: {}", e),
                    )
                    .await?;
                } else {
                    mark_transient_failure(pool, queued.id, backoff).await?;
                }
                continue;
            }
            Ok(true) => {}
        }

        let subject = match &queued.subject {
            Some(s) => s,
            None => {
                tracing::warn!("Queue item {} has no subject, skipping", queued.id);
                continue;
            }
        };
        let html_content = match &queued.html_content {
            Some(h) => h,
            None => {
                tracing::warn!("Queue item {} has no html_content, skipping", queued.id);
                continue;
            }
        };

        let unsubscribe_url = format!(
            "{}/unsubscribe?token={}",
            config.site_url, queued.unsubscribe_token
        );
        let unsubscribe_post_url = format!(
            "{}/api/v1/unsubscribe/{}",
            config.public_url, queued.unsubscribe_token
        );

        let html = match crate::templates::render_newsletter(
            html_content,
            &unsubscribe_url,
            &config.site_url,
            queued.newsletter.as_deref(),
            queued.preview_text.as_deref(),
        ) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Template render error for queue item {}: {}", queued.id, e);
                mark_permanent_failure(pool, queued.id, &format!("Template error: {}", e)).await?;
                continue;
            }
        };

        match email_service
            .send_newsletter(
                &queued.email,
                subject,
                &html,
                &unsubscribe_url,
                &unsubscribe_post_url,
            )
            .await
        {
            Ok(()) => {
                mark_sent(pool, queued.id).await?;
            }
            Err(e) => {
                let error_msg = e.to_string();
                if is_permanent_error(&error_msg) {
                    tracing::error!(
                        "Permanent send failure for queue item {} to {}: {}",
                        queued.id,
                        queued.email,
                        error_msg
                    );
                    mark_permanent_failure(pool, queued.id, &error_msg).await?;
                } else if queued.attempts + 1 >= MAX_ATTEMPTS {
                    tracing::error!(
                        "Max retries reached for queue item {} to {}: {}",
                        queued.id,
                        queued.email,
                        error_msg
                    );
                    mark_permanent_failure(pool, queued.id, &error_msg).await?;
                } else {
                    let backoff = next_attempt_delay(queued.attempts + 1);
                    tracing::warn!(
                        "Transient failure for queue item {} to {} (attempt {}), retrying in {:?}: {}",
                        queued.id,
                        queued.email,
                        queued.attempts + 1,
                        backoff,
                        error_msg
                    );
                    mark_transient_failure(pool, queued.id, backoff).await?;
                }
            }
        }

        tokio::time::sleep(delay).await;
    }

    Ok(())
}

fn is_permanent_error(error_msg: &str) -> bool {
    let permanent_patterns = [
        "MessageRejected",
        "InvalidParameterValue",
        "AccountSendingPaused",
    ];
    permanent_patterns.iter().any(|p| error_msg.contains(p))
}

fn next_attempt_delay(attempts: i32) -> Duration {
    let base_secs = 30u64;
    let delay_secs = base_secs * 2u64.pow(attempts.max(0) as u32);
    Duration::from_secs(delay_secs.min(600))
}

async fn fetch_pending(pool: &PgPool, limit: i64) -> Result<Vec<QueuedEmail>, sqlx::Error> {
    sqlx::query_as::<_, QueuedEmail>(
        r#"SELECT es.id, es.subscriber_id, es.post_slug, es.newsletter, es.unsubscribe_token,
                  es.subject, es.html_content, es.preview_text, es.attempts, s.email
           FROM email_sends es
           JOIN subscribers s ON s.id = es.subscriber_id
           WHERE es.sent_at IS NULL
             AND es.send_error IS NULL
             AND es.next_attempt_at IS NOT NULL
             AND es.next_attempt_at <= NOW()
           ORDER BY es.next_attempt_at ASC
           LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

async fn mark_sent(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_sends SET sent_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn mark_permanent_failure(pool: &PgPool, id: i64, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE email_sends SET send_error = $2, attempts = attempts + 1 WHERE id = $1")
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;
    Ok(())
}

async fn mark_transient_failure(
    pool: &PgPool,
    id: i64,
    backoff: Duration,
) -> Result<(), sqlx::Error> {
    let next_attempt: DateTime<Utc> =
        Utc::now() + chrono::Duration::seconds(backoff.as_secs() as i64);
    sqlx::query(
        "UPDATE email_sends SET attempts = attempts + 1, next_attempt_at = $2 WHERE id = $1",
    )
    .bind(id)
    .bind(next_attempt)
    .execute(pool)
    .await?;
    Ok(())
}
