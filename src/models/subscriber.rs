use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing;
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImportSubscriberEntry {
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    pub newsletters: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportResult {
    pub created: i64,
    pub updated: i64,
    pub total: i64,
}

/// Maps a newsletter name to the corresponding subscription column.
/// Returns None for invalid newsletter names. Safe for SQL interpolation
/// because the return values are hardcoded string literals.
fn newsletter_column(newsletter: &str) -> Option<&'static str> {
    match newsletter {
        "postcard" => Some("subscribed_postcard"),
        "contraption" => Some("subscribed_contraption"),
        "workshop" => Some("subscribed_workshop"),
        _ => None,
    }
}

impl Subscriber {
    pub async fn find_by_id(pool: &PgPool, id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM subscribers WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

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

    pub async fn count_eligible(
        pool: &PgPool,
        newsletter: &str,
        post_slug: &str,
    ) -> Result<i64, sqlx::Error> {
        let column = newsletter_column(newsletter);
        if column.is_none() {
            return Ok(0);
        }
        let query = format!(
            r#"SELECT COUNT(*) FROM subscribers s
               WHERE s.confirmed_at IS NOT NULL
                 AND s.{} = TRUE
                 AND NOT EXISTS (
                   SELECT 1 FROM email_sends es
                   WHERE es.subscriber_id = s.id AND es.post_slug = $1
                 )"#,
            column.unwrap()
        );
        let row: (i64,) = sqlx::query_as(&query)
            .bind(post_slug)
            .fetch_one(pool)
            .await?;
        Ok(row.0)
    }

    pub async fn find_eligible_ids(
        pool: &PgPool,
        newsletter: &str,
        post_slug: &str,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let column = newsletter_column(newsletter);
        if column.is_none() {
            return Ok(vec![]);
        }
        let query = format!(
            r#"SELECT s.id FROM subscribers s
               WHERE s.confirmed_at IS NOT NULL
                 AND s.{} = TRUE
                 AND NOT EXISTS (
                   SELECT 1 FROM email_sends es
                   WHERE es.subscriber_id = s.id AND es.post_slug = $1
                 )"#,
            column.unwrap()
        );
        let rows: Vec<(i64,)> = sqlx::query_as(&query)
            .bind(post_slug)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    pub async fn count_active(pool: &PgPool) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM subscribers
               WHERE confirmed_at IS NOT NULL
                 AND (subscribed_postcard = TRUE
                   OR subscribed_contraption = TRUE
                   OR subscribed_workshop = TRUE)"#,
        )
        .fetch_one(pool)
        .await?;
        Ok(row.0)
    }

    pub async fn bulk_import(
        pool: &PgPool,
        entries: &[ImportSubscriberEntry],
    ) -> Result<ImportResult, sqlx::Error> {
        if entries.is_empty() {
            return Ok(ImportResult {
                created: 0,
                updated: 0,
                total: 0,
            });
        }

        let emails: Vec<String> = entries.iter().map(|e| e.email.to_lowercase()).collect();
        let names: Vec<Option<String>> = entries.iter().map(|e| e.name.clone()).collect();
        let sources: Vec<Option<String>> = entries.iter().map(|e| e.source.clone()).collect();
        let sub_postcard: Vec<bool> = entries
            .iter()
            .map(|e| e.newsletters.iter().any(|n| n == "postcard"))
            .collect();
        let sub_contraption: Vec<bool> = entries
            .iter()
            .map(|e| e.newsletters.iter().any(|n| n == "contraption"))
            .collect();
        let sub_workshop: Vec<bool> = entries
            .iter()
            .map(|e| e.newsletters.iter().any(|n| n == "workshop"))
            .collect();

        let rows: Vec<(bool,)> = sqlx::query_as(
            r#"INSERT INTO subscribers (email, name, source, confirmed_at,
                   subscribed_postcard, subscribed_contraption, subscribed_workshop)
               SELECT * FROM unnest(
                   $1::text[], $2::text[], $3::text[],
                   array_fill(NOW()::timestamptz, ARRAY[$7::int]),
                   $4::bool[], $5::bool[], $6::bool[]
               )
               ON CONFLICT (email) DO UPDATE SET
                   subscribed_postcard = subscribers.subscribed_postcard OR EXCLUDED.subscribed_postcard,
                   subscribed_contraption = subscribers.subscribed_contraption OR EXCLUDED.subscribed_contraption,
                   subscribed_workshop = subscribers.subscribed_workshop OR EXCLUDED.subscribed_workshop,
                   name = COALESCE(subscribers.name, EXCLUDED.name),
                   source = COALESCE(subscribers.source, EXCLUDED.source),
                   confirmed_at = COALESCE(subscribers.confirmed_at, NOW()),
                   updated_at = NOW()
               RETURNING (xmax = 0)"#,
        )
        .bind(&emails)
        .bind(&names)
        .bind(&sources)
        .bind(&sub_postcard)
        .bind(&sub_contraption)
        .bind(&sub_workshop)
        .bind(entries.len() as i32)
        .fetch_all(pool)
        .await?;

        let created = rows.iter().filter(|(is_insert,)| *is_insert).count() as i64;
        let updated = rows.len() as i64 - created;

        tracing::info!(created, updated, total = rows.len(), "Bulk import complete");

        Ok(ImportResult {
            created,
            updated,
            total: rows.len() as i64,
        })
    }

    pub async fn delete_with_data(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM logins WHERE subscriber_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM email_sends WHERE subscriber_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM subscribers WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
