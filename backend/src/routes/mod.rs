pub mod dashboard;
pub mod docker;
pub mod groups;
pub mod icons;
pub mod logs;
pub mod monitors;
pub mod notifications;
pub mod services;
pub mod settings;
pub mod version;

use axum::{Json, Router, routing::get};

use crate::{auth, state::AppState};

pub fn public() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/login", axum::routing::post(auth::login))
        .merge(dashboard::router())
        .merge(icons::public_router())
}

pub fn protected() -> Router<AppState> {
    Router::new()
        .route(
            "/api/auth/me",
            get(auth::me).delete(auth::logout).post(auth::logout),
        )
        .route("/api/auth/logout", axum::routing::post(auth::logout))
        .route(
            "/api/auth/credentials",
            axum::routing::put(auth::update_credentials),
        )
        .merge(groups::router())
        .merge(icons::router())
        .merge(logs::router())
        .merge(docker::router())
        .merge(monitors::router())
        .merge(notifications::router())
        .merge(services::router())
        .merge(settings::router())
        .merge(version::router())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "app": "ServiceCompass",
        "version": version::current_version()
    }))
}
