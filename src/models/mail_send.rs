use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct MailSend {
    pub id: i64,
    pub stripe_customer_id: String,
    pub customer_email: Option<String>,
    pub post_slug: String,
    pub newsletter: String,
    pub idempotency_key: String,
    pub lob_letter_id: Option<String>,
    pub pages: Option<i32>,
    pub double_sided: bool,
    pub carrier: Option<String>,
    pub send_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

impl MailSend {
    pub async fn create(
        pool: &PgPool,
        stripe_customer_id: &str,
        customer_email: Option<&str>,
        post_slug: &str,
        newsletter: &str,
        idempotency_key: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO mail_sends (stripe_customer_id, customer_email, post_slug, newsletter, idempotency_key)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING *"#,
        )
        .bind(stripe_customer_id)
        .bind(customer_email)
        .bind(post_slug)
        .bind(newsletter)
        .bind(idempotency_key)
        .fetch_one(pool)
        .await
    }

    pub async fn mark_sent(
        pool: &PgPool,
        id: i64,
        lob_letter_id: &str,
        pages: i32,
        double_sided: bool,
        carrier: &str,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE mail_sends
               SET lob_letter_id = $2, pages = $3, double_sided = $4, carrier = $5, sent_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(lob_letter_id)
        .bind(pages)
        .bind(double_sided)
        .bind(carrier)
        .fetch_one(pool)
        .await
    }

    pub async fn record_error(pool: &PgPool, id: i64, error: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE mail_sends SET send_error = $2
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(error)
        .fetch_one(pool)
        .await
    }

    pub async fn exists_by_idempotency_key(pool: &PgPool, key: &str) -> Result<bool, sqlx::Error> {
        let row: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM mail_sends WHERE idempotency_key = $1)")
                .bind(key)
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }

    pub async fn count_sent_for_slug(pool: &PgPool, post_slug: &str) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM mail_sends WHERE post_slug = $1 AND sent_at IS NOT NULL",
        )
        .bind(post_slug)
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }
}
