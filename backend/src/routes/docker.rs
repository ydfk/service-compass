use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    docker::{client, scanner},
    error::{AppError, AppResult},
    models::docker::{
        AddCandidateInput, DockerCandidate, DockerEndpointInput, DockerEndpointRow,
        DockerEndpointView,
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/docker/endpoints", get(list).post(create))
        .route(
            "/api/docker/endpoints/{id}",
            get(get_one).put(update).delete(remove),
        )
        .route("/api/docker/endpoints/{id}/test", post(test))
        .route("/api/docker/endpoints/{id}/scan", post(scan))
        .route("/api/docker/endpoints/{id}/candidates", get(candidates))
        .route("/api/docker/candidates/add", post(add_candidate))
}

async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<DockerEndpointView>>> {
    let rows =
        sqlx::query_as::<_, DockerEndpointRow>("SELECT * FROM docker_endpoints ORDER BY name")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<DockerEndpointView>> {
    Ok(Json(find(&state, &id).await?.into()))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<DockerEndpointInput>,
) -> AppResult<Json<DockerEndpointView>> {
    validate(&input, true)?;
    let id = Uuid::new_v4().to_string();
    save(&state, &id, &input, true).await?;
    tracing::info!(endpoint_id = %id, name = input.name.trim(), "Docker Endpoint 创建完成");
    Ok(Json(find(&state, &id).await?.into()))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<DockerEndpointInput>,
) -> AppResult<Json<DockerEndpointView>> {
    validate(&input, false)?;
    if !save(&state, &id, &input, false).await? {
        return Err(AppError::NotFound);
    }
    tracing::info!(endpoint_id = %id, name = input.name.trim(), "Docker Endpoint 更新完成");
    Ok(Json(find(&state, &id).await?.into()))
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM docker_endpoints WHERE id = ?")
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
) -> AppResult<Json<serde_json::Value>> {
    let endpoint = find(&state, &id).await?;
    tracing::info!(endpoint_id = %id, "开始测试 Docker Endpoint");
    let docker = client::connect(&state, &endpoint).map_err(AppError::Internal)?;
    let response = docker.ping().await.map_err(anyhow::Error::from)?;
    tracing::info!(endpoint_id = %id, response = %response, "Docker Endpoint 测试成功");
    Ok(Json(
        serde_json::json!({ "ok": true, "response": response }),
    ))
}

async fn scan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<DockerCandidate>>> {
    let endpoint = find(&state, &id).await?;
    tracing::info!(endpoint_id = %id, "开始扫描 Docker 候选");
    let rows = scanner::scan(&state, &endpoint)
        .await
        .map_err(AppError::Internal)?;
    tracing::info!(endpoint_id = %id, count = rows.len(), "Docker 候选扫描完成");
    Ok(Json(rows))
}

async fn candidates(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<DockerCandidate>>> {
    find(&state, &id).await?;
    let values: Vec<String> = sqlx::query_scalar(
        "SELECT candidates_json FROM docker_scan_cache WHERE endpoint_id = ? ORDER BY container_name",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    let rows = values
        .into_iter()
        .filter_map(|value| serde_json::from_str(&value).ok())
        .collect();
    Ok(Json(rows))
}

async fn add_candidate(
    State(state): State<AppState>,
    Json(input): Json<AddCandidateInput>,
) -> AppResult<Json<serde_json::Value>> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT candidates_json FROM docker_scan_cache WHERE endpoint_id = ? AND container_id = ?",
    )
    .bind(&input.endpoint_id)
    .bind(&input.container_id)
    .fetch_optional(&state.pool)
    .await?;
    let candidate: DockerCandidate = serde_json::from_str(&raw.ok_or(AppError::NotFound)?)
        .map_err(|error| AppError::Internal(error.into()))?;
    let service_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let name = input.name.as_deref().unwrap_or(&candidate.suggested_name);
    let local_url = input.local_url.or(candidate.local_url);
    let public_url = input.public_url.or(candidate.public_url);
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO services (id, group_id, name, icon_type, icon_value, local_url, public_url, \
         docker_endpoint_id, docker_container_id, docker_name, docker_image, \
         docker_compose_project, docker_compose_service, enabled, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, 'selfhst', ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, 0, ?, ?)",
    )
    .bind(&service_id)
    .bind(&input.group_id)
    .bind(name)
    .bind(input.icon_value.unwrap_or(candidate.suggested_icon))
    .bind(&local_url)
    .bind(&public_url)
    .bind(&input.endpoint_id)
    .bind(&candidate.container_id)
    .bind(&candidate.container_name)
    .bind(&candidate.image)
    .bind(&candidate.compose_project)
    .bind(&candidate.compose_service)
    .bind(&now)
    .bind(&now)
    .execute(&mut *transaction)
    .await?;
    if input.create_monitor {
        create_default_monitor(&mut transaction, &service_id, name, &now).await?;
    }
    transaction.commit().await?;
    Ok(Json(serde_json::json!({ "service_id": service_id })))
}

async fn create_default_monitor(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    service_id: &str,
    service_name: &str,
    now: &str,
) -> AppResult<()> {
    let monitor_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO monitors (id, service_id, name, monitor_type, target_url_mode, created_at, updated_at) \
         VALUES (?, ?, ?, 'http', 'local', ?, ?)",
    )
    .bind(&monitor_id)
    .bind(service_id)
    .bind(format!("{service_name} HTTP"))
    .bind(now)
    .bind(now)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO monitor_states (monitor_id, next_check_at, updated_at) VALUES (?, ?, ?)",
    )
    .bind(monitor_id)
    .bind(now)
    .bind(now)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn find(state: &AppState, id: &str) -> AppResult<DockerEndpointRow> {
    sqlx::query_as::<_, DockerEndpointRow>("SELECT * FROM docker_endpoints WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)
}

async fn save(
    state: &AppState,
    id: &str,
    input: &DockerEndpointInput,
    insert: bool,
) -> AppResult<bool> {
    let existing = if insert {
        (None, None, None)
    } else {
        sqlx::query_as(
            "SELECT tls_ca_secret, tls_cert_secret, tls_key_secret FROM docker_endpoints WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?
    };
    let ca = encrypt_optional(state, input.tls_ca.as_deref())?.or(existing.0);
    let cert = encrypt_optional(state, input.tls_cert.as_deref())?.or(existing.1);
    let key = encrypt_optional(state, input.tls_key.as_deref())?.or(existing.2);
    let now = Utc::now().to_rfc3339();
    let result = if insert {
        sqlx::query(
            "INSERT INTO docker_endpoints (id, name, endpoint_type, endpoint_url, tls_enabled, tls_ca_secret, \
             tls_cert_secret, tls_key_secret, lan_host, public_host_hint, enabled, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(input.name.trim())
        .bind(&input.endpoint_type)
        .bind(&input.endpoint_url)
        .bind(input.tls_enabled)
        .bind(ca)
        .bind(cert)
        .bind(key)
        .bind(&input.lan_host)
        .bind(&input.public_host_hint)
        .bind(input.enabled)
        .bind(&now)
        .bind(&now)
        .execute(&state.pool)
        .await?
    } else {
        sqlx::query(
            "UPDATE docker_endpoints SET name = ?, endpoint_type = ?, endpoint_url = ?, tls_enabled = ?, \
             tls_ca_secret = ?, tls_cert_secret = ?, tls_key_secret = ?, lan_host = ?, public_host_hint = ?, \
             enabled = ?, updated_at = ? WHERE id = ?",
        )
        .bind(input.name.trim())
        .bind(&input.endpoint_type)
        .bind(&input.endpoint_url)
        .bind(input.tls_enabled)
        .bind(ca)
        .bind(cert)
        .bind(key)
        .bind(&input.lan_host)
        .bind(&input.public_host_hint)
        .bind(input.enabled)
        .bind(&now)
        .bind(id)
        .execute(&state.pool)
        .await?
    };
    Ok(result.rows_affected() > 0)
}

fn encrypt_optional(state: &AppState, value: Option<&str>) -> AppResult<Option<String>> {
    value
        .filter(|item| !item.trim().is_empty())
        .map(|item| state.secrets.encrypt(item).map_err(AppError::Internal))
        .transpose()
}

fn validate(input: &DockerEndpointInput, creating: bool) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("Endpoint 名称不能为空".into()));
    }
    match input.endpoint_type.as_str() {
        "local_socket" if input.endpoint_url.starts_with("unix://") => {}
        "remote_tcp" if input.endpoint_url.starts_with("tcp://") => {}
        _ => return Err(AppError::Validation("Endpoint 类型或地址格式无效".into())),
    }
    if input.endpoint_type == "remote_tcp"
        && input.tls_enabled
        && creating
        && (input.tls_ca.as_deref().unwrap_or_default().is_empty()
            || input.tls_cert.as_deref().unwrap_or_default().is_empty()
            || input.tls_key.as_deref().unwrap_or_default().is_empty())
    {
        return Err(AppError::Validation(
            "首次创建 TLS Endpoint 时必须填写 CA、Cert 和 Key".into(),
        ));
    }
    Ok(())
}
