use std::time::Duration;

use sqlx::PgPool;

use crate::config::Config;
use crate::models::email_suppression::EmailSuppression;

pub async fn run(pool: PgPool, config: Config) {
    if config.email_backend != "ses" {
        tracing::info!("Suppression sync disabled (email backend is not SES)");
        return;
    }

    tracing::info!("Starting SES suppression sync (hourly)");
    let mut interval = tokio::time::interval(Duration::from_secs(3600));

    loop {
        interval.tick().await;
        if let Err(e) = sync_suppressions(&pool, &config).await {
            tracing::error!("Suppression sync error: {}", e);
        }
    }
}

async fn sync_suppressions(
    pool: &PgPool,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(config.aws_region.clone()))
        .load()
        .await;
    let client = aws_sdk_sesv2::Client::new(&sdk_config);

    let mut next_token: Option<String> = None;
    let mut total = 0u64;

    loop {
        let mut req = client.list_suppressed_destinations();
        if let Some(token) = &next_token {
            req = req.next_token(token);
        }

        let resp = req.send().await?;

        for dest in resp.suppressed_destination_summaries() {
            let email = dest.email_address();
            let reason = format!("{:?}", dest.reason());

            EmailSuppression::upsert(pool, email, &reason, Some("ses_suppression_list")).await?;
            total += 1;
        }

        next_token = resp.next_token().map(|s| s.to_string());
        if next_token.is_none() {
            break;
        }
    }

    tracing::info!("Suppression sync complete: {} entries synced", total);
    Ok(())
}
