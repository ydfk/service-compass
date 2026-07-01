use axum::{Json, Router, extract::State, routing::get};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    access,
    error::{AppError, AppResult},
    logs,
    state::AppState,
};

#[derive(Deserialize)]
struct SettingsInput {
    retention_days: i64,
    #[serde(default = "default_retention_days")]
    log_retention_days: i64,
    cert_expiry_warning_days: i64,
    notification_cooldown_sec: i64,
    #[serde(default = "default_dashboard_refresh_interval")]
    dashboard_refresh_interval_sec: i64,
    #[serde(default = "default_anonymous_access_cidrs")]
    anonymous_access_cidrs: String,
}

const fn default_retention_days() -> i64 {
    30
}

const fn default_dashboard_refresh_interval() -> i64 {
    30
}

fn default_anonymous_access_cidrs() -> String {
    access::DEFAULT_ANONYMOUS_CIDRS.to_owned()
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings).put(update_settings))
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let retention_days = setting_i64(&state, "retention_days", 30).await?;
    let log_retention_days = setting_i64(&state, "log_retention_days", 30).await?;
    let cert_expiry_warning_days = setting_i64(&state, "cert_expiry_warning_days", 30).await?;
    let notification_cooldown_sec = setting_i64(&state, "notification_cooldown_sec", 300).await?;
    let dashboard_refresh_interval_sec =
        setting_i64(&state, "dashboard_refresh_interval_sec", 30).await?;
    let anonymous_access_cidrs = setting_string(
        &state,
        "anonymous_access_cidrs",
        access::DEFAULT_ANONYMOUS_CIDRS,
    )
    .await?;
    Ok(Json(serde_json::json!({
        "retention_days": retention_days,
        "log_retention_days": log_retention_days,
        "cert_expiry_warning_days": cert_expiry_warning_days,
        "notification_cooldown_sec": notification_cooldown_sec,
        "dashboard_refresh_interval_sec": dashboard_refresh_interval_sec,
        "anonymous_access_cidrs": anonymous_access_cidrs
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
    if !(1..=365).contains(&input.log_retention_days) {
        return Err(AppError::Validation(
            "系统日志保留天数必须在 1 到 365 之间".into(),
        ));
    }
    if !(1..=365).contains(&input.cert_expiry_warning_days) {
        return Err(AppError::Validation(
            "证书到期提醒天数必须在 1 到 365 之间".into(),
        ));
    }
    if !(0..=86_400).contains(&input.notification_cooldown_sec) {
        return Err(AppError::Validation(
            "通知冷却时间必须在 0 到 86400 秒之间".into(),
        ));
    }
    if !(5..=3600).contains(&input.dashboard_refresh_interval_sec) {
        return Err(AppError::Validation(
            "首页刷新间隔必须在 5 到 3600 秒之间".into(),
        ));
    }
    access::validate_cidrs(&input.anonymous_access_cidrs)?;
    save_setting(&state, "retention_days", input.retention_days).await?;
    save_setting(&state, "log_retention_days", input.log_retention_days).await?;
    save_setting(
        &state,
        "cert_expiry_warning_days",
        input.cert_expiry_warning_days,
    )
    .await?;
    save_setting(
        &state,
        "notification_cooldown_sec",
        input.notification_cooldown_sec,
    )
    .await?;
    save_setting(
        &state,
        "dashboard_refresh_interval_sec",
        input.dashboard_refresh_interval_sec,
    )
    .await?;
    save_setting_string(
        &state,
        "anonymous_access_cidrs",
        &input.anonymous_access_cidrs,
    )
    .await?;
    sync_existing_values(&state, &input).await?;
    logs::set_retention_days(input.log_retention_days)
        .map_err(|error| AppError::Internal(error.into()))?;
    Ok(Json(serde_json::json!({
        "retention_days": input.retention_days,
        "log_retention_days": input.log_retention_days,
        "cert_expiry_warning_days": input.cert_expiry_warning_days,
        "notification_cooldown_sec": input.notification_cooldown_sec,
        "dashboard_refresh_interval_sec": input.dashboard_refresh_interval_sec,
        "anonymous_access_cidrs": input.anonymous_access_cidrs
    })))
}

async fn sync_existing_values(state: &AppState, input: &SettingsInput) -> AppResult<()> {
    sqlx::query("UPDATE monitors SET cert_warning_days = ? WHERE monitor_type = 'cert'")
        .bind(input.cert_expiry_warning_days)
        .execute(&state.pool)
        .await?;
    sqlx::query("UPDATE notification_rules SET cooldown_sec = ?")
        .bind(input.notification_cooldown_sec)
        .execute(&state.pool)
        .await?;
    let cutoff = (Utc::now() - chrono::Duration::days(input.retention_days.max(1))).to_rfc3339();
    sqlx::query("DELETE FROM monitor_checks WHERE checked_at < ?")
        .bind(cutoff)
        .execute(&state.pool)
        .await?;
    Ok(())
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

async fn setting_string(state: &AppState, key: &str, fallback: &str) -> AppResult<String> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&state.pool)
        .await?;
    Ok(value.unwrap_or_else(|| fallback.to_owned()))
}

async fn save_setting(state: &AppState, key: &str, value: i64) -> AppResult<()> {
    save_setting_string(state, key, &value.to_string()).await
}

async fn save_setting_string(state: &AppState, key: &str, value: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(key)
    .bind(value.trim())
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await?;
    Ok(())
}
