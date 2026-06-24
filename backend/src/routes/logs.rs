use axum::{Json, Router, extract::Query, routing::get};
use serde::Deserialize;

use crate::{logs, state::AppState};

#[derive(Deserialize)]
struct LogsQuery {
    #[serde(default = "default_limit")]
    limit: usize,
}

const fn default_limit() -> usize {
    200
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/logs", get(list))
}

async fn list(Query(query): Query<LogsQuery>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "logs": logs::recent(query.limit) }))
}
