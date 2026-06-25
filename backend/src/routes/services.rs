use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::Utc;
use reqwest::Url;
use uuid::Uuid;

use crate::{
    db::UNGROUPED_GROUP_ID,
    error::{AppError, AppResult},
    models::{
        group::ReorderItem,
        monitor::MonitorInput,
        service::{Service, ServiceInput},
    },
    routes::monitors,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/services", get(list).post(create))
        .route("/api/services/reorder", post(reorder))
        .route(
            "/api/services/{id}",
            get(get_one).put(update).delete(remove),
        )
        .route("/api/services/{id}/test-open", post(test_open))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<Service>>> {
    let services =
        sqlx::query_as::<_, Service>("SELECT * FROM services ORDER BY group_id, sort_order, name")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(services))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Service>> {
    Ok(Json(find(&state, &id).await?))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<ServiceInput>,
) -> AppResult<Json<Service>> {
    validate(&input)?;
    let id = Uuid::new_v4().to_string();
    persist(&state, &id, &input, true).await?;
    sync_service_monitors(&state, &id, &input).await?;
    tracing::info!(service_id = %id, name = input.name.trim(), "服务创建完成");
    Ok(Json(find(&state, &id).await?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ServiceInput>,
) -> AppResult<Json<Service>> {
    validate(&input)?;
    if !persist(&state, &id, &input, false).await? {
        return Err(AppError::NotFound);
    }
    sync_service_monitors(&state, &id, &input).await?;
    tracing::info!(service_id = %id, name = input.name.trim(), "服务更新完成");
    Ok(Json(find(&state, &id).await?))
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM services WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    tracing::info!(service_id = %id, "服务删除完成");
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn reorder(
    State(state): State<AppState>,
    Json(items): Json<Vec<ReorderItem>>,
) -> AppResult<Json<serde_json::Value>> {
    let count = items.len();
    let mut transaction = state.pool.begin().await?;
    for item in items {
        sqlx::query("UPDATE services SET sort_order = ?, updated_at = ? WHERE id = ?")
            .bind(item.sort_order)
            .bind(Utc::now().to_rfc3339())
            .bind(item.id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    tracing::info!(count, "服务排序更新完成");
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn test_open(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let service = find(&state, &id).await?;
    let url = match service.preferred_url_mode.as_str() {
        "public" => service.public_url.or(service.local_url),
        _ => service.local_url.or(service.public_url),
    };
    Ok(Json(serde_json::json!({ "url": url })))
}

async fn find(state: &AppState, id: &str) -> AppResult<Service> {
    sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn persist(
    state: &AppState,
    id: &str,
    input: &ServiceInput,
    insert: bool,
) -> AppResult<bool> {
    let now = Utc::now().to_rfc3339();
    let result = if insert {
        sqlx::query(
            "INSERT INTO services (id, group_id, name, description, icon_type, icon_value, local_url, \
             public_url, preferred_url_mode, docker_endpoint_id, docker_container_id, docker_name, \
             docker_image, docker_compose_project, docker_compose_service, enabled, sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(group_id(input))
        .bind(input.name.trim())
        .bind(&input.description)
        .bind(&input.icon_type)
        .bind(&input.icon_value)
        .bind(&input.local_url)
        .bind(&input.public_url)
        .bind(&input.preferred_url_mode)
        .bind(&input.docker_endpoint_id)
        .bind(&input.docker_container_id)
        .bind(&input.docker_name)
        .bind(&input.docker_image)
        .bind(&input.docker_compose_project)
        .bind(&input.docker_compose_service)
        .bind(input.enabled)
        .bind(input.sort_order)
        .bind(&now)
        .bind(&now)
        .execute(&state.pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE services SET group_id = ?, name = ?, description = ?, icon_type = ?, icon_value = ?, \
             local_url = ?, public_url = ?, preferred_url_mode = ?, docker_endpoint_id = ?, \
             docker_container_id = ?, docker_name = ?, docker_image = ?, docker_compose_project = ?, \
             docker_compose_service = ?, enabled = ?, sort_order = ?, updated_at = ? WHERE id = ?",
        )
        .bind(group_id(input))
        .bind(input.name.trim())
        .bind(&input.description)
        .bind(&input.icon_type)
        .bind(&input.icon_value)
        .bind(&input.local_url)
        .bind(&input.public_url)
        .bind(&input.preferred_url_mode)
        .bind(&input.docker_endpoint_id)
        .bind(&input.docker_container_id)
        .bind(&input.docker_name)
        .bind(&input.docker_image)
        .bind(&input.docker_compose_project)
        .bind(&input.docker_compose_service)
        .bind(input.enabled)
        .bind(input.sort_order)
        .bind(&now)
        .bind(id)
        .execute(&state.pool)
        .await?
    };
    Ok(result.rows_affected() > 0)
}

fn group_id(input: &ServiceInput) -> &str {
    input
        .group_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(UNGROUPED_GROUP_ID)
}

async fn sync_service_monitors(
    state: &AppState,
    service_id: &str,
    input: &ServiceInput,
) -> AppResult<()> {
    let legacy_monitor = input.create_monitor.then(|| {
        MonitorInput::service_http(
            service_id.to_string(),
            format!("{} HTTP", input.name.trim()),
            input.monitor_type.clone(),
            input.monitor_target_url_mode.clone(),
        )
    });
    let http_monitor = input.monitor.as_ref().or(legacy_monitor.as_ref());
    monitors::sync_http_for_service(state, service_id, input.name.trim(), http_monitor).await?;
    let cert_monitor = http_monitor.and_then(|monitor| cert_monitor_input(input, monitor));
    monitors::sync_cert_for_service(
        state,
        service_id,
        input.name.trim(),
        cert_monitor.as_ref(),
        input.cert_expiry_notify,
    )
    .await?;
    let docker_enabled = input.docker_endpoint_id.is_some() && input.docker_container_id.is_some();
    monitors::sync_docker_for_service(state, service_id, input.name.trim(), docker_enabled).await?;
    Ok(())
}

fn cert_monitor_input(input: &ServiceInput, monitor: &MonitorInput) -> Option<MonitorInput> {
    let url = cert_source_url(input, monitor)?;
    let parsed = Url::parse(&url).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    let domain = parsed.host_str()?.to_owned();
    let mut cert = monitor.clone();
    cert.monitor_type = "cert".into();
    cert.domain = Some(domain);
    cert.cert_port = i64::from(parsed.port_or_known_default().unwrap_or(443));
    cert.notify_on_down = true;
    cert.notify_on_warning = true;
    cert.notify_on_recovery = true;
    Some(cert)
}

fn cert_source_url(input: &ServiceInput, monitor: &MonitorInput) -> Option<String> {
    if monitor.target_url_mode == "custom" {
        return monitor.target_url.clone();
    }
    match monitor.target_url_mode.as_str() {
        "local" => input.local_url.clone().or_else(|| input.public_url.clone()),
        "public" => input.public_url.clone().or_else(|| input.local_url.clone()),
        _ => None,
    }
}

fn validate(input: &ServiceInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("服务名称不能为空".into()));
    }
    if !matches!(input.preferred_url_mode.as_str(), "local" | "public") {
        return Err(AppError::Validation(
            "访问模式必须是 local 或 public".into(),
        ));
    }
    Ok(())
}
