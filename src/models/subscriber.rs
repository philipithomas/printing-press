use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Subscriber {
    pub id: i64,
    pub uuid: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub subscribed_postcard: bool,
    pub subscribed_contraption: bool,
    pub subscribed_workshop: bool,
    pub source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSubscriberRequest {
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub google_verified: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSubscriberRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub subscribed_postcard: Option<bool>,
    #[serde(default)]
    pub subscribed_contraption: Option<bool>,
    #[serde(default)]
    pub subscribed_workshop: Option<bool>,
}

impl Subscriber {
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM subscribers WHERE email = $1")
            .bind(email.to_lowercase())
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_uuid(pool: &PgPool, uuid: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM subscribers WHERE uuid = $1")
            .bind(uuid)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &PgPool,
        email: &str,
        name: Option<&str>,
        source: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"INSERT INTO subscribers (email, name, source)
               VALUES ($1, $2, $3)
               RETURNING *"#,
        )
        .bind(email.to_lowercase())
        .bind(name)
        .bind(source)
        .fetch_one(pool)
        .await
    }

    pub async fn confirm(pool: &PgPool, id: i64) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE subscribers SET confirmed_at = NOW(), updated_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        uuid: Uuid,
        req: &UpdateSubscriberRequest,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE subscribers SET
                name = COALESCE($2, name),
                subscribed_postcard = COALESCE($3, subscribed_postcard),
                subscribed_contraption = COALESCE($4, subscribed_contraption),
                subscribed_workshop = COALESCE($5, subscribed_workshop),
                updated_at = NOW()
               WHERE uuid = $1
               RETURNING *"#,
        )
        .bind(uuid)
        .bind(&req.name)
        .bind(req.subscribed_postcard)
        .bind(req.subscribed_contraption)
        .bind(req.subscribed_workshop)
        .fetch_optional(pool)
        .await
    }

    pub async fn unsubscribe_all(pool: &PgPool, id: i64) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"UPDATE subscribers SET
                subscribed_postcard = FALSE,
                subscribed_contraption = FALSE,
                subscribed_workshop = FALSE,
                updated_at = NOW()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }
}
