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
    cert_expiry_warning_days: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let retention_days = setting_i64(&state, "retention_days", 30).await?;
    let cert_expiry_warning_days = setting_i64(&state, "cert_expiry_warning_days", 30).await?;
    Ok(Json(serde_json::json!({
        "retention_days": retention_days,
        "cert_expiry_warning_days": cert_expiry_warning_days
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
    if !(1..=365).contains(&input.cert_expiry_warning_days) {
        return Err(AppError::Validation(
            "证书到期提醒天数必须在 1 到 365 之间".into(),
        ));
    }
    save_setting(&state, "retention_days", input.retention_days).await?;
    save_setting(
        &state,
        "cert_expiry_warning_days",
        input.cert_expiry_warning_days,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "retention_days": input.retention_days,
        "cert_expiry_warning_days": input.cert_expiry_warning_days
    })))
}

async fn setting_i64(state: &AppState, key: &str, fallback: i64) -> AppResult<i64> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&state.pool)
        .await?;
    Ok(value
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback))
}

async fn save_setting(state: &AppState, key: &str, value: i64) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value.to_string())
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(())
}
