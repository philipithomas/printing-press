use axum_test::TestServer;
use printing_press::{config::Config, db, routes, state::AppState};
use sqlx::PgPool;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

#[allow(dead_code)]
pub struct TestApp {
    pub server: TestServer,
    pub pool: PgPool,
    pub api_key: String,
    _container: testcontainers::ContainerAsync<Postgres>,
}

impl TestApp {
    pub async fn new() -> Self {
        let container = Postgres::default()
            .with_tag("17-alpine")
            .start()
            .await
            .unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

        let pool = db::connect(&database_url).await.unwrap();
        db::migrate(&pool).await.unwrap();

        let api_key = "test-api-key".to_string();
        let config = Config {
            database_url,
            m2m_api_key: api_key.clone(),
            aws_region: "us-east-1".to_string(),
            ses_from_email: "test@example.com".to_string(),
            site_url: "http://localhost:3000".to_string(),
            host: "127.0.0.1".to_string(),
            port: 0,
            email_backend: "smtp".to_string(),
            smtp_host: "localhost".to_string(),
            smtp_port: 1025,
            ses_rate_per_second: 14,
            lob_api_key: String::new(),
            stripe_secret_key: String::new(),
            stripe_product_id: "prod_TNjLOf5BZLeoKW".to_string(),
            chromium_path: String::new(),
        };

        let state = AppState::new(pool.clone(), config).await;
        let app = routes::router(state);
        let server = TestServer::new(app);

        TestApp {
            server,
            pool,
            api_key,
            _container: container,
        }
    }
}
