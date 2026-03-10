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
    pub fn new(pool: PgPool, config: Config) -> Self {
        let email_service = EmailService::new(config.clone());
        Self {
            pool,
            config,
            email_service,
        }
    }
}
