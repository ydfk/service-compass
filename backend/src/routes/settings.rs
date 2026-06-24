use axum::{Json, Router, extract::State, routing::get};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Deserialize)]
struct SettingsInput {
    retention_days: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let retention_days: Option<String> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'retention_days'")
            .fetch_optional(&state.pool)
            .await?;
    Ok(Json(serde_json::json!({
        "retention_days": retention_days.and_then(|value| value.parse().ok()).unwrap_or(30)
    })))
}

async fn update_settings(
    State(state): State<AppState>,
    Json(input): Json<SettingsInput>,
) -> AppResult<Json<serde_json::Value>> {
    if !(1..=365).contains(&input.retention_days) {
        return Err(AppError::Validation(
            "历史保留天数必须在 1 到 365 之间".into(),
        ));
    }
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES ('retention_days', ?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(input.retention_days.to_string())
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(Json(
        serde_json::json!({ "retention_days": input.retention_days }),
    ))
}
