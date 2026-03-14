use printing_press::{config::Config, db, services::{queue_worker, suppression_sync}, state::AppState};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "printing_press=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load()?;
    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;

    let state = AppState::new(pool, config.clone()).await;
    let app = printing_press::routes::router(state.clone());

    // Spawn background queue worker
    let worker_pool = state.pool.clone();
    let worker_email = state.email_service.clone();
    let worker_config = state.config.clone();
    let worker_mx = state.mx_validator.clone();
    tokio::spawn(async move {
        queue_worker::run(worker_pool, worker_email, worker_config, worker_mx).await;
    });

    // Spawn SES suppression sync
    let sync_pool = state.pool.clone();
    let sync_config = state.config.clone();
    tokio::spawn(async move {
        suppression_sync::run(sync_pool, sync_config).await;
    });

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
