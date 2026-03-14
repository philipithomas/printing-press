use sqlx::PgPool;

pub struct EmailSuppression;

impl EmailSuppression {
    pub async fn is_suppressed(pool: &PgPool, email: &str) -> Result<bool, sqlx::Error> {
        let row: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM email_suppressions WHERE email = $1)")
                .bind(email)
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }

    pub async fn upsert(
        pool: &PgPool,
        email: &str,
        reason: &str,
        source: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO email_suppressions (email, reason, source)
               VALUES ($1, $2, $3)
               ON CONFLICT (email) DO UPDATE SET reason = $2, source = $3"#,
        )
        .bind(email)
        .bind(reason)
        .bind(source)
        .execute(pool)
        .await?;
        Ok(())
    }
}
