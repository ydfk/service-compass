pub mod access;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod docker;
pub mod error;
pub mod icon;
pub mod logs;
pub mod maintenance;
pub mod models;
pub mod monitor;
pub mod notify;
pub mod routes;
pub mod state;

use std::path::{Path, PathBuf};

use axum::{Router, middleware};
use state::AppState;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub fn app(state: AppState, static_dir: &Path) -> Router {
    let public_content = routes::public_content().route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth::allow_private_network_or_auth,
    ));
    let protected = routes::protected().route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_auth,
    ));
    let index = PathBuf::from(static_dir).join("index.html");
    let static_files = ServeDir::new(static_dir).not_found_service(ServeFile::new(index));

    Router::new()
        .merge(routes::public())
        .merge(public_content)
        .merge(protected)
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
