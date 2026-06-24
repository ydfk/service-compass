use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
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
        .route("/api/monitors/{id}/test", post(test))
        .route("/api/monitors/{id}/checks", get(checks))
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
    Ok(view)
}

async fn save(state: &AppState, id: &str, input: &MonitorInput, insert: bool) -> AppResult<bool> {
    let existing_secret: Option<String> = if insert {
        None
    } else {
        sqlx::query_scalar("SELECT auth_password_secret FROM monitors WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
            .flatten()
    };
    let password_secret = input
        .auth_password
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| state.secrets.encrypt(value))
        .transpose()
        .map_err(AppError::Internal)?
        .or(existing_secret);
    let now = Utc::now().to_rfc3339();
    let sql = if insert {
        "INSERT INTO monitors (id, service_id, name, monitor_type, target_url, target_url_mode, method, expected_status_min, expected_status_max, keyword, interval_sec, timeout_sec, retries, retry_interval_sec, follow_redirects, tls_verify, auth_type, auth_username, auth_password_secret, domain, record_type, expected_value, cert_port, cert_warning_days, cert_critical_days, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    } else {
        "UPDATE monitors SET service_id = ?, name = ?, monitor_type = ?, target_url = ?, target_url_mode = ?, method = ?, expected_status_min = ?, expected_status_max = ?, keyword = ?, interval_sec = ?, timeout_sec = ?, retries = ?, retry_interval_sec = ?, follow_redirects = ?, tls_verify = ?, auth_type = ?, auth_username = ?, auth_password_secret = ?, domain = ?, record_type = ?, expected_value = ?, cert_port = ?, cert_warning_days = ?, cert_critical_days = ?, enabled = ?, updated_at = ? WHERE id = ?"
    };
    let mut query = sqlx::query(sql);
    if insert {
        query = query.bind(id);
    }
    query = bind_input(query, input, password_secret);
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

fn validate(input: &MonitorInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("监控名称不能为空".into()));
    }
    if !matches!(
        input.monitor_type.as_str(),
        "http" | "http_keyword" | "dns" | "cert" | "docker"
    ) {
        return Err(AppError::Validation("监控类型无效".into()));
    }
    let is_http = matches!(input.monitor_type.as_str(), "http" | "http_keyword");
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
    if input.interval_sec < 5
        || input.timeout_sec < 1
        || input.expected_status_min > input.expected_status_max
    {
        return Err(AppError::Validation(
            "检查间隔、超时或状态码范围无效".into(),
        ));
    }
    Ok(())
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
        "SELECT id FROM monitors WHERE service_id = ? AND monitor_type IN ('http', 'http_keyword') \
         ORDER BY created_at LIMIT 1",
    )
    .bind(service_id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(input) = input else {
        sqlx::query(
            "DELETE FROM monitors WHERE service_id = ? AND monitor_type IN ('http', 'http_keyword')",
        )
        .bind(service_id)
        .execute(&state.pool)
        .await?;
        return Ok(());
    };
    let mut monitor = input.clone();
    monitor.service_id = Some(service_id.to_string());
    if monitor.name.trim().is_empty() {
        monitor.name = format!("{service_name} HTTP");
    }
    validate(&monitor)?;
    if let Some(id) = existing {
        save(state, &id, &monitor, false).await?;
    } else {
        create_record(state, &monitor).await?;
    }
    Ok(())
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
