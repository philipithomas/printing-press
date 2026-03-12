use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct EmailSend {
    pub id: i64,
    pub subscriber_id: i64,
    pub post_slug: String,
    pub unsubscribe_token: Uuid,
    pub send_error: Option<String>,
    pub triggered_unsubscribe_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub subject: Option<String>,
    pub html_content: Option<String>,
    pub newsletter: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub next_attempt_at: Option<DateTime<Utc>>,
}

impl EmailSend {
    pub async fn create(
        pool: &PgPool,
        subscriber_id: i64,
        post_slug: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO email_sends (subscriber_id, post_slug)
               VALUES ($1, $2)
               RETURNING *"#,
        )
        .bind(subscriber_id)
        .bind(post_slug)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_unsubscribe_token(
        pool: &PgPool,
        token: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM email_sends WHERE unsubscribe_token = $1")
            .bind(token)
            .fetch_optional(pool)
            .await
    }

    pub async fn mark_unsubscribed(pool: &PgPool, id: i64) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE email_sends SET triggered_unsubscribe_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn record_error(pool: &PgPool, id: i64, error: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE email_sends SET send_error = $2
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(error)
        .fetch_one(pool)
        .await
    }

    pub async fn count_by_slug(pool: &PgPool, slug: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM email_sends WHERE post_slug = $1")
            .bind(slug)
            .fetch_one(pool)
            .await?;
        Ok(row.0)
    }

    pub async fn bulk_create_queued(
        pool: &PgPool,
        subscriber_ids: &[i64],
        post_slug: &str,
        newsletter: &str,
        subject: &str,
        html_content: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"INSERT INTO email_sends (subscriber_id, post_slug, newsletter, subject, html_content, next_attempt_at)
               SELECT unnest($1::bigint[]), $2, $3, $4, $5, NOW()
               "#,
        )
        .bind(subscriber_ids)
        .bind(post_slug)
        .bind(newsletter)
        .bind(subject)
        .bind(html_content)
        .execute(pool)
        .await?;
        Ok(result.rows_affected() as i64)
    }
}
