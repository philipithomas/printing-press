use crate::config::Config;
use crate::services::email_service::EmailService;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub email_service: EmailService,
}

impl AppState {
    pub async fn new(pool: PgPool, config: Config) -> Self {
        let email_service = EmailService::new(&config).await;
        Self {
            pool,
            config,
            email_service,
        }
    }
}
