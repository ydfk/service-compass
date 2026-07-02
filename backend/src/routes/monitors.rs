use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch, post},
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::monitor::{MonitorCheck, MonitorInput, MonitorRow, MonitorView},
    monitor,
    state::AppState,
};

#[derive(Deserialize)]
struct ChecksQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

#[derive(Deserialize)]
pub struct MonitorNotificationInput {
    notify_enabled: bool,
    notification_channel_ids: Vec<String>,
}

const fn default_limit() -> i64 {
    100
}

type MonitorStateSummary = (
    String,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<String>,
);

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/monitors", get(list).post(create))
        .route(
            "/api/monitors/{id}",
            get(get_one).put(update).delete(remove),
        )
        .route(
            "/api/monitors/{id}/notification",
            patch(update_notification),
        )
        .route("/api/monitors/{id}/test", post(test))
        .route("/api/monitors/{id}/checks", get(checks))
}

async fn update_notification(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<MonitorNotificationInput>,
) -> AppResult<Json<MonitorView>> {
    let row = find(&state, &id).await?;
    save_notification_rules(
        &state,
        &id,
        input.notify_enabled,
        &input.notification_channel_ids,
        true,
        true,
        true,
    )
    .await?;
    Ok(Json(view_with_state(&state, row).await?))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<MonitorView>>> {
    let rows = sqlx::query_as::<_, MonitorRow>("SELECT * FROM monitors ORDER BY name")
        .fetch_all(&state.pool)
        .await?;
    let mut views = Vec::with_capacity(rows.len());
    for row in rows {
        views.push(view_with_state(&state, row).await?);
    }
    Ok(Json(views))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<MonitorView>> {
    let row = find(&state, &id).await?;
    Ok(Json(view_with_state(&state, row).await?))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<MonitorInput>,
) -> AppResult<Json<MonitorView>> {
    let id = create_record(&state, &input).await?;
    sync_notification_rules(&state, &id, &input).await?;
    Ok(Json(
        view_with_state(&state, find(&state, &id).await?).await?,
    ))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<MonitorInput>,
) -> AppResult<Json<MonitorView>> {
    validate(&input)?;
    if !save(&state, &id, &input, false).await? {
        return Err(AppError::NotFound);
    }
    sync_notification_rules(&state, &id, &input).await?;
    Ok(Json(
        view_with_state(&state, find(&state, &id).await?).await?,
    ))
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM monitors WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn test(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<monitor::CheckResult>> {
    let row = find(&state, &id).await?;
    Ok(Json(monitor::scheduler::run(&state, &row).await))
}

async fn checks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ChecksQuery>,
) -> AppResult<Json<Vec<MonitorCheck>>> {
    find(&state, &id).await?;
    let rows = sqlx::query_as::<_, MonitorCheck>(
        "SELECT * FROM monitor_checks WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT ?",
    )
    .bind(id)
    .bind(query.limit.clamp(1, 500))
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

async fn find(state: &AppState, id: &str) -> AppResult<MonitorRow> {
    sqlx::query_as::<_, MonitorRow>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn view_with_state(state: &AppState, row: MonitorRow) -> AppResult<MonitorView> {
    let state_row: Option<MonitorStateSummary> = sqlx::query_as(
        "SELECT s.current_status, s.last_checked_at, s.last_latency_ms, s.last_error, \
             (SELECT c.extra_json FROM monitor_checks c WHERE c.monitor_id = s.monitor_id \
              ORDER BY c.checked_at DESC LIMIT 1) \
             FROM monitor_states s WHERE s.monitor_id = ?",
    )
    .bind(&row.id)
    .fetch_optional(&state.pool)
    .await?;
    let mut view = MonitorView::from_row(row);
    if let Some((status, checked_at, latency, error, extra)) = state_row {
        view.current_status = status;
        view.last_checked_at = checked_at;
        view.last_latency_ms = latency;
        view.last_error = error;
        view.last_extra_json = extra;
    }
    view.recent_checks = sqlx::query_as::<_, MonitorCheck>(
        "SELECT * FROM monitor_checks WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 5",
    )
    .bind(&view.id)
    .fetch_all(&state.pool)
    .await?;
    view.recent_statuses = sqlx::query_scalar::<_, String>(
        "SELECT status FROM monitor_checks WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 30",
    )
    .bind(&view.id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .rev()
    .collect();
    apply_notification_settings(state, &mut view).await?;
    Ok(view)
}

async fn apply_notification_settings(state: &AppState, view: &mut MonitorView) -> AppResult<()> {
    let rules: Vec<(String, bool, bool, bool, i64, bool)> = sqlx::query_as(
        "SELECT channel_id, notify_on_down, notify_on_recovery, notify_on_warning, cooldown_sec, enabled \
         FROM notification_rules WHERE monitor_id = ? ORDER BY created_at",
    )
    .bind(&view.id)
    .fetch_all(&state.pool)
    .await?;
    let Some(first) = rules.first() else {
        return Ok(());
    };
    let notify_on_down = first.1;
    let notify_on_recovery = first.2;
    let notify_on_warning = first.3;
    let cooldown_sec = first.4;
    view.notify_enabled = rules.iter().any(|item| item.5);
    view.notification_channel_ids = rules.into_iter().map(|item| item.0).collect();
    view.notify_on_down = notify_on_down;
    view.notify_on_recovery = notify_on_recovery;
    view.notify_on_warning = notify_on_warning;
    view.notification_cooldown_sec = cooldown_sec;
    Ok(())
}

async fn save(state: &AppState, id: &str, input: &MonitorInput, insert: bool) -> AppResult<bool> {
    let mut input = input.clone();
    apply_global_settings(state, &mut input).await?;
    let existing_secrets: Option<(Option<String>, Option<String>, Option<String>)> = if insert {
        None
    } else {
        sqlx::query_as(
            "SELECT auth_password_secret, request_body_secret, request_headers_secret \
             FROM monitors WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
    };
    let password_secret = input
        .auth_password
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| state.secrets.encrypt(value))
        .transpose()
        .map_err(AppError::Internal)?
        .or_else(|| existing_secrets.as_ref().and_then(|item| item.0.clone()));
    let request_body_secret = input
        .request_body
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| state.secrets.encrypt(value))
        .transpose()
        .map_err(AppError::Internal)?
        .or_else(|| existing_secrets.as_ref().and_then(|item| item.1.clone()));
    let request_headers_secret = input
        .request_headers
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| state.secrets.encrypt(value))
        .transpose()
        .map_err(AppError::Internal)?
        .or_else(|| existing_secrets.as_ref().and_then(|item| item.2.clone()));
    let now = Utc::now().to_rfc3339();
    let sql = if insert {
        "INSERT INTO monitors (id, service_id, name, monitor_type, target_url, target_url_mode, method, expected_status_min, expected_status_max, keyword, interval_sec, timeout_sec, retries, retry_interval_sec, follow_redirects, tls_verify, request_body_type, request_body_secret, request_headers_secret, auth_type, auth_username, auth_password_secret, domain, record_type, expected_value, cert_port, cert_warning_days, cert_critical_days, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    } else {
        "UPDATE monitors SET service_id = ?, name = ?, monitor_type = ?, target_url = ?, target_url_mode = ?, method = ?, expected_status_min = ?, expected_status_max = ?, keyword = ?, interval_sec = ?, timeout_sec = ?, retries = ?, retry_interval_sec = ?, follow_redirects = ?, tls_verify = ?, request_body_type = ?, request_body_secret = ?, request_headers_secret = ?, auth_type = ?, auth_username = ?, auth_password_secret = ?, domain = ?, record_type = ?, expected_value = ?, cert_port = ?, cert_warning_days = ?, cert_critical_days = ?, enabled = ?, updated_at = ? WHERE id = ?"
    };
    let mut query = sqlx::query(sql);
    if insert {
        query = query.bind(id);
    }
    query = bind_input(
        query,
        &input,
        password_secret,
        request_body_secret,
        request_headers_secret,
    );
    if insert {
        query = query.bind(&now).bind(&now);
    } else {
        query = query.bind(&now).bind(id);
    }
    Ok(query.execute(&state.pool).await?.rows_affected() > 0)
}

fn bind_input<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    input: &'q MonitorInput,
    password_secret: Option<String>,
    request_body_secret: Option<String>,
    request_headers_secret: Option<String>,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    query
        .bind(&input.service_id)
        .bind(input.name.trim())
        .bind(&input.monitor_type)
        .bind(&input.target_url)
        .bind(&input.target_url_mode)
        .bind(&input.method)
        .bind(input.expected_status_min)
        .bind(input.expected_status_max)
        .bind(&input.keyword)
        .bind(input.interval_sec)
        .bind(input.timeout_sec)
        .bind(input.retries)
        .bind(input.retry_interval_sec)
        .bind(input.follow_redirects)
        .bind(input.tls_verify)
        .bind(&input.request_body_type)
        .bind(request_body_secret)
        .bind(request_headers_secret)
        .bind(&input.auth_type)
        .bind(&input.auth_username)
        .bind(password_secret)
        .bind(&input.domain)
        .bind(&input.record_type)
        .bind(&input.expected_value)
        .bind(input.cert_port)
        .bind(input.cert_warning_days)
        .bind(input.cert_critical_days)
        .bind(input.enabled)
}

async fn apply_global_settings(state: &AppState, input: &mut MonitorInput) -> AppResult<()> {
    if input.monitor_type == "cert" {
        input.cert_warning_days = setting_i64(state, "cert_expiry_warning_days", 30).await?;
    }
    Ok(())
}

fn validate(input: &MonitorInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("监控名称不能为空".into()));
    }
    if !matches!(
        input.monitor_type.as_str(),
        "http" | "http_keyword" | "dns" | "cert" | "docker" | "postgres"
    ) {
        return Err(AppError::Validation("监控类型无效".into()));
    }
    let is_http = matches!(input.monitor_type.as_str(), "http" | "http_keyword");
    if is_http && !matches!(input.method.as_str(), "GET" | "HEAD" | "POST") {
        return Err(AppError::Validation(
            "HTTP 请求方法只支持 GET、HEAD、POST".into(),
        ));
    }
    if is_http && !matches!(input.request_body_type.as_str(), "json" | "form") {
        return Err(AppError::Validation("请求体编码必须是 json 或 form".into()));
    }
    if is_http
        && input.target_url_mode == "custom"
        && input.target_url.as_deref().unwrap_or_default().is_empty()
    {
        return Err(AppError::Validation("目标 URL 不能为空".into()));
    }
    if input.monitor_type == "http_keyword"
        && input.keyword.as_deref().unwrap_or_default().is_empty()
    {
        return Err(AppError::Validation("关键字不能为空".into()));
    }
    if matches!(input.monitor_type.as_str(), "dns" | "cert")
        && input.domain.as_deref().unwrap_or_default().is_empty()
    {
        return Err(AppError::Validation("域名不能为空".into()));
    }
    if input.monitor_type == "docker" && input.service_id.is_none() {
        return Err(AppError::Validation("Docker 监控必须关联服务".into()));
    }
    if input.monitor_type == "postgres" {
        validate_postgres(input)?;
    }
    if input.interval_sec < 5
        || input.timeout_sec < 1
        || input.expected_status_min > input.expected_status_max
        || input.notification_cooldown_sec < 0
    {
        return Err(AppError::Validation(
            "检查间隔、超时或状态码范围无效".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_input(input: &MonitorInput) -> AppResult<()> {
    validate(input)
}

fn validate_postgres(input: &MonitorInput) -> AppResult<()> {
    if input
        .target_url
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(AppError::Validation("PostgreSQL 主机不能为空".into()));
    }
    if !(1..=65535).contains(&input.cert_port) {
        return Err(AppError::Validation("PostgreSQL 端口无效".into()));
    }
    if input
        .domain
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(AppError::Validation("PostgreSQL 数据库名不能为空".into()));
    }
    if input
        .auth_username
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(AppError::Validation("PostgreSQL 用户名不能为空".into()));
    }
    let query = input
        .expected_value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("SELECT 1");
    if !query_is_readonly(query) {
        return Err(AppError::Validation(
            "PostgreSQL 检查 SQL 仅支持 SELECT、SHOW 或 WITH 查询".into(),
        ));
    }
    Ok(())
}

fn query_is_readonly(query: &str) -> bool {
    let normalized = query.trim_start().to_lowercase();
    normalized.starts_with("select ")
        || normalized == "select"
        || normalized.starts_with("show ")
        || normalized == "show"
        || normalized.starts_with("with ")
        || normalized == "with"
}

async fn create_record(state: &AppState, input: &MonitorInput) -> AppResult<String> {
    validate(input)?;
    let id = Uuid::new_v4().to_string();
    save(state, &id, input, true).await?;
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO monitor_states (monitor_id, next_check_at, updated_at) VALUES (?, ?, ?)",
    )
    .bind(&id)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await?;
    Ok(id)
}

pub(crate) async fn sync_http_for_service(
    state: &AppState,
    service_id: &str,
    service_name: &str,
    input: Option<&MonitorInput>,
) -> AppResult<()> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM monitors WHERE service_id = ? AND monitor_type IN ('http', 'http_keyword', 'postgres') \
         ORDER BY created_at LIMIT 1",
    )
    .bind(service_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(input) = input else {
        sqlx::query(
            "DELETE FROM monitors WHERE service_id = ? AND monitor_type IN ('http', 'http_keyword', 'postgres')",
        )
        .bind(service_id)
        .execute(&state.pool)
        .await?;
        return Ok(());
    };
    let mut monitor = input.clone();
    monitor.service_id = Some(service_id.to_string());
    if monitor.name.trim().is_empty() {
        monitor.name = format!(
            "{} {}",
            service_name,
            primary_monitor_label(&monitor.monitor_type)
        );
    }
    validate(&monitor)?;
    if let Some(id) = existing {
        save(state, &id, &monitor, false).await?;
        sync_notification_rules(state, &id, &monitor).await?;
    } else {
        let id = create_record(state, &monitor).await?;
        sync_notification_rules(state, &id, &monitor).await?;
    }
    Ok(())
}

fn primary_monitor_label(monitor_type: &str) -> &str {
    match monitor_type {
        "postgres" => "PostgreSQL",
        "http_keyword" => "HTTP 关键字",
        _ => "HTTP",
    }
}

pub(crate) async fn sync_docker_for_service(
    state: &AppState,
    service_id: &str,
    service_name: &str,
    enabled: bool,
) -> AppResult<()> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM monitors WHERE service_id = ? AND monitor_type = 'docker' LIMIT 1",
    )
    .bind(service_id)
    .fetch_optional(&state.pool)
    .await?;
    if !enabled {
        if let Some(id) = existing {
            sqlx::query("DELETE FROM monitors WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await?;
        }
        return Ok(());
    }
    if let Some(id) = existing {
        sqlx::query("UPDATE monitors SET name = ?, enabled = 1, updated_at = ? WHERE id = ?")
            .bind(format!("{service_name} Docker"))
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(&state.pool)
            .await?;
        return Ok(());
    }
    let input = MonitorInput::service_http(
        service_id.to_string(),
        format!("{service_name} Docker"),
        "docker".into(),
        "custom".into(),
    );
    create_record(state, &input).await?;
    Ok(())
}

pub(crate) async fn sync_cert_for_service(
    state: &AppState,
    service_id: &str,
    service_name: &str,
    monitor: Option<&MonitorInput>,
    enabled: bool,
) -> AppResult<()> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM monitors WHERE service_id = ? AND monitor_type = 'cert' LIMIT 1",
    )
    .bind(service_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(monitor) = monitor.filter(|_| enabled) else {
        if let Some(id) = existing {
            sqlx::query("DELETE FROM monitors WHERE id = ?")
                .bind(id)
                .execute(&state.pool)
                .await?;
        }
        return Ok(());
    };
    let mut input = monitor.clone();
    input.service_id = Some(service_id.to_string());
    input.name = format!("{service_name} 证书");
    input.monitor_type = "cert".into();
    input.domain = monitor.domain.clone();
    input.enabled = true;
    validate(&input)?;
    if let Some(id) = existing {
        save(state, &id, &input, false).await?;
        sync_notification_rules(state, &id, &input).await?;
    } else {
        let id = create_record(state, &input).await?;
        sync_notification_rules(state, &id, &input).await?;
    }
    Ok(())
}

async fn sync_notification_rules(
    state: &AppState,
    monitor_id: &str,
    input: &MonitorInput,
) -> AppResult<()> {
    save_notification_rules(
        state,
        monitor_id,
        input.notify_enabled,
        &input.notification_channel_ids,
        input.notify_on_down,
        input.notify_on_recovery,
        input.notify_on_warning,
    )
    .await
}

pub(crate) async fn save_notification_rules(
    state: &AppState,
    monitor_id: &str,
    enabled: bool,
    channel_ids: &[String],
    notify_on_down: bool,
    notify_on_recovery: bool,
    notify_on_warning: bool,
) -> AppResult<()> {
    sqlx::query("DELETE FROM notification_rules WHERE monitor_id = ?")
        .bind(monitor_id)
        .execute(&state.pool)
        .await?;
    if !enabled || channel_ids.is_empty() {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    let cooldown_sec = setting_i64(state, "notification_cooldown_sec", 300).await?;
    for channel_id in channel_ids {
        sqlx::query(
            "INSERT INTO notification_rules (id, monitor_id, channel_id, notify_on_down, notify_on_recovery, \
             notify_on_warning, cooldown_sec, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(monitor_id)
        .bind(channel_id)
        .bind(notify_on_down)
        .bind(notify_on_recovery)
        .bind(notify_on_warning)
        .bind(cooldown_sec)
        .bind(&now)
        .bind(&now)
        .execute(&state.pool)
        .await?;
    }
    Ok(())
}

pub(crate) async fn sync_notification_for_service_monitor(
    state: &AppState,
    service_id: &str,
    monitor_type: &str,
    enabled: bool,
    channel_ids: &[String],
) -> AppResult<()> {
    let monitor_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM monitors WHERE service_id = ? AND monitor_type = ? LIMIT 1",
    )
    .bind(service_id)
    .bind(monitor_type)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(id) = monitor_id {
        save_notification_rules(state, &id, enabled, channel_ids, true, true, true).await?;
    }
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
