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
}
