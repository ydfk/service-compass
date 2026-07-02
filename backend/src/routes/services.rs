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
    icon::local,
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
    Json(mut input): Json<ServiceInput>,
) -> AppResult<Json<Service>> {
    localize_service_icon(&state, &mut input).await;
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
    Json(mut input): Json<ServiceInput>,
) -> AppResult<Json<Service>> {
    localize_service_icon(&state, &mut input).await;
    validate(&input)?;
    if !persist(&state, &id, &input, false).await? {
        return Err(AppError::NotFound);
    }
    sync_service_monitors(&state, &id, &input).await?;
    tracing::info!(service_id = %id, name = input.name.trim(), "服务更新完成");
    Ok(Json(find(&state, &id).await?))
}

async fn localize_service_icon(state: &AppState, input: &mut ServiceInput) {
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    else {
        return;
    };
    match local::localize_icon_value(
        &state.config,
        &client,
        &input.icon_type,
        input.icon_value.as_deref(),
    )
    .await
    {
        Ok(Some((icon_type, icon_value))) => {
            input.icon_type = icon_type;
            input.icon_value = Some(icon_value);
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(
                ?error,
                name = input.name.trim(),
                "服务图标本地化失败，保留原图标"
            );
        }
    }
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
    let url = service.public_url.or(service.local_url);
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
             public_url, docker_endpoint_id, docker_container_id, docker_name, \
             docker_image, docker_compose_project, docker_compose_service, enabled, sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(group_id(input))
        .bind(input.name.trim())
        .bind(&input.description)
        .bind(&input.icon_type)
        .bind(&input.icon_value)
        .bind(clean_url(input.local_url.as_deref()))
        .bind(clean_url(input.public_url.as_deref()))
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
             local_url = ?, public_url = ?, docker_endpoint_id = ?, \
             docker_container_id = ?, docker_name = ?, docker_image = ?, docker_compose_project = ?, \
             docker_compose_service = ?, enabled = ?, sort_order = ?, updated_at = ? WHERE id = ?",
        )
        .bind(group_id(input))
        .bind(input.name.trim())
        .bind(&input.description)
        .bind(&input.icon_type)
        .bind(&input.icon_value)
        .bind(clean_url(input.local_url.as_deref()))
        .bind(clean_url(input.public_url.as_deref()))
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

fn clean_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || matches!(value, "http://" | "https://") {
        None
    } else {
        Some(value.to_owned())
    }
}

async fn sync_service_monitors(
    state: &AppState,
    service_id: &str,
    input: &ServiceInput,
) -> AppResult<()> {
    let mut legacy_monitor = input.create_monitor.then(|| {
        MonitorInput::service_http(
            service_id.to_string(),
            format!(
                "{} {}",
                input.name.trim(),
                primary_monitor_label(&input.monitor_type)
            ),
            input.monitor_type.clone(),
            input.monitor_target_url_mode.clone(),
        )
    });
    let notification = service_notification(input);
    let mut monitor_with_notify = input.monitor.clone();
    if let (Some(monitor), Some(notification)) = (&mut monitor_with_notify, notification.as_ref()) {
        apply_notification(monitor, notification);
    }
    if let (Some(monitor), Some(notification)) = (&mut legacy_monitor, notification.as_ref()) {
        apply_notification(monitor, notification);
    }
    let http_monitor = monitor_with_notify.as_ref().or(legacy_monitor.as_ref());
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
    let docker_enabled = input.docker_endpoint_id.is_some()
        && (input.docker_container_id.is_some()
            || input.docker_name.is_some()
            || (input.docker_compose_project.is_some() && input.docker_compose_service.is_some()));
    monitors::sync_docker_for_service(state, service_id, input.name.trim(), docker_enabled).await?;
    if let Some(notification) = notification {
        if docker_enabled {
            monitors::sync_notification_for_service_monitor(
                state,
                service_id,
                "docker",
                notification.enabled,
                &notification.channel_ids,
            )
            .await?;
        }
        if input.cert_expiry_notify {
            monitors::sync_notification_for_service_monitor(
                state,
                service_id,
                "cert",
                notification.enabled,
                &notification.channel_ids,
            )
            .await?;
        }
    }
    Ok(())
}

struct ServiceNotification {
    enabled: bool,
    channel_ids: Vec<String>,
}

fn service_notification(input: &ServiceInput) -> Option<ServiceNotification> {
    if input.status_notify_enabled.is_none() && input.status_notification_channel_ids.is_none() {
        return input.monitor.as_ref().map(|monitor| ServiceNotification {
            enabled: monitor.notify_enabled,
            channel_ids: monitor.notification_channel_ids.clone(),
        });
    }
    Some(ServiceNotification {
        enabled: input.status_notify_enabled.unwrap_or(false),
        channel_ids: input
            .status_notification_channel_ids
            .clone()
            .unwrap_or_default(),
    })
}

fn apply_notification(monitor: &mut MonitorInput, notification: &ServiceNotification) {
    monitor.notify_enabled = notification.enabled;
    monitor.notification_channel_ids = notification.channel_ids.clone();
    monitor.notify_on_down = true;
    monitor.notify_on_recovery = true;
    monitor.notify_on_warning = true;
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
    validate_service_notification(input)?;
    if input.create_monitor {
        let mut monitor = input.monitor.clone().unwrap_or_else(|| {
            MonitorInput::service_http(
                "pending".into(),
                format!(
                    "{} {}",
                    input.name.trim(),
                    primary_monitor_label(&input.monitor_type)
                ),
                input.monitor_type.clone(),
                input.monitor_target_url_mode.clone(),
            )
        });
        if monitor.name.trim().is_empty() {
            monitor.name = format!(
                "{} {}",
                input.name.trim(),
                primary_monitor_label(&monitor.monitor_type)
            );
        }
        validate_monitor_source(input, &monitor)?;
        monitors::validate_input(&monitor)?;
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

fn validate_service_notification(input: &ServiceInput) -> AppResult<()> {
    let notification = service_notification(input);
    if notification
        .as_ref()
        .is_some_and(|item| item.enabled && item.channel_ids.is_empty())
    {
        return Err(AppError::Validation("开启通知时必须选择通知通道".into()));
    }
    Ok(())
}

fn validate_monitor_source(input: &ServiceInput, monitor: &MonitorInput) -> AppResult<()> {
    if matches!(monitor.monitor_type.as_str(), "http" | "http_keyword") {
        if monitor.target_url_mode == "custom"
            && monitor
                .target_url
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(AppError::Validation("目标 URL 不能为空".into()));
        }
        if matches!(monitor.target_url_mode.as_str(), "local" | "public")
            && input
                .local_url
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
            && input
                .public_url
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(AppError::Validation(
                "监控使用服务地址时，内网地址或外网地址至少填写一个".into(),
            ));
        }
    }
    Ok(())
}
