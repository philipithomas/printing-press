use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Login {
    pub id: i64,
    pub subscriber_id: i64,
    pub token: String,
    pub token_type: String,
    pub email_sent_at: Option<DateTime<Utc>>,
    pub verified_at: Option<DateTime<Utc>>,
    pub expired_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub attempts: i32,
    pub locked_at: Option<DateTime<Utc>>,
}

const MAX_VERIFICATION_ATTEMPTS: i32 = 5;

impl Login {
    pub async fn create(
        pool: &PgPool,
        subscriber_id: i64,
        token: &str,
        token_type: &str,
        expired_at: DateTime<Utc>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO logins (subscriber_id, token, token_type, expired_at)
               VALUES ($1, $2, $3, $4)
               RETURNING *"#,
        )
        .bind(subscriber_id)
        .bind(token)
        .bind(token_type)
        .bind(expired_at)
        .fetch_one(pool)
        .await
    }

    pub async fn find_valid_by_token(
        pool: &PgPool,
        token: &str,
        token_type: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM logins
               WHERE token = $1
               AND token_type = $2
               AND verified_at IS NULL
               AND locked_at IS NULL
               AND expired_at > NOW()"#,
        )
        .bind(token)
        .bind(token_type)
        .fetch_optional(pool)
        .await
    }

    /// Increment the attempt counter for all unexpired code logins for a subscriber.
    /// Locks the login if attempts exceed MAX_VERIFICATION_ATTEMPTS.
    pub async fn increment_attempts_for_subscriber(
        pool: &PgPool,
        subscriber_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE logins
               SET attempts = attempts + 1,
                   locked_at = CASE
                       WHEN attempts + 1 >= $2 THEN NOW()
                       ELSE locked_at
                   END
               WHERE subscriber_id = $1
               AND token_type = 'code'
               AND verified_at IS NULL
               AND locked_at IS NULL
               AND expired_at > NOW()"#,
        )
        .bind(subscriber_id)
        .bind(MAX_VERIFICATION_ATTEMPTS)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_verified(pool: &PgPool, id: i64) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE logins SET verified_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn mark_email_sent(pool: &PgPool, id: i64) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE logins SET email_sent_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }
}
