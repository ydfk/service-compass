use std::sync::Arc;

use service_compass_backend::{app, config::Config, db, maintenance, state::AppState};
use tokio::signal;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(Config::from_env()?);
    service_compass_backend::logs::init(&config.log_dir, 30)?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_ansi(false)
        .with_writer(service_compass_backend::logs::LogWriterFactory)
        .init();

    maintenance::restore::apply_pending_restore(&config)?;
    let pool = db::connect(&config.database_url).await?;
    let log_retention_days = sqlx::query_scalar::<_, i64>(
        "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'log_retention_days'",
    )
    .fetch_optional(&pool)
    .await?
    .unwrap_or(30);
    service_compass_backend::logs::set_retention_days(log_retention_days)?;
    let state = AppState::new(pool, Arc::clone(&config))?;
    service_compass_backend::monitor::scheduler::start(state.clone());
    maintenance::backup::start_scheduler(state.clone());
    let router = app(state, &config.static_dir);
    let listener = tokio::net::TcpListener::bind(&config.bind).await?;

    info!(address = %config.bind, "ServiceCompass 已启动");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("无法监听 Ctrl+C 信号");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("无法监听终止信号")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
