use std::time::Instant;

use anyhow::{Context, Result};
use bollard::{Docker, query_parameters::ListContainersOptionsBuilder};
use serde_json::Value;

use crate::{
    docker::client,
    models::{docker::DockerEndpointRow, monitor::MonitorRow},
    monitor::CheckResult,
    state::AppState,
};

type DockerBinding = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub async fn check(state: &AppState, monitor: &MonitorRow) -> CheckResult {
    check_once(state, monitor)
        .await
        .unwrap_or_else(|error| CheckResult::down(error.to_string()))
}

async fn check_once(state: &AppState, monitor: &MonitorRow) -> Result<CheckResult> {
    let service_id = monitor
        .service_id
        .as_deref()
        .context("Docker 监控未关联服务")?;
    let binding: Option<DockerBinding> = sqlx::query_as(
        "SELECT docker_endpoint_id, docker_container_id, docker_name, docker_compose_project, docker_compose_service \
         FROM services WHERE id = ?",
    )
    .bind(service_id)
    .fetch_optional(&state.pool)
    .await?;
    let (endpoint_id, container_id, container_name, compose_project, compose_service) =
        binding.context("服务不存在")?;
    let endpoint_id = endpoint_id.context("服务没有 Docker Endpoint 关联")?;
    if container_id.is_none()
        && container_name.is_none()
        && (compose_project.is_none() || compose_service.is_none())
    {
        anyhow::bail!("服务没有 Docker 容器关联");
    }
    let endpoint = sqlx::query_as::<_, DockerEndpointRow>(
        "SELECT * FROM docker_endpoints WHERE id = ? AND enabled = 1",
    )
    .bind(endpoint_id)
    .fetch_optional(&state.pool)
    .await?
    .context("Docker Endpoint 不存在或已禁用")?;

    tracing::info!(
        monitor_id = %monitor.id,
        endpoint_id = %endpoint.id,
        old_container_id = container_id.as_deref().unwrap_or(""),
        "开始检查 Docker 容器状态"
    );
    let started = Instant::now();
    let docker = client::connect(state, &endpoint)?;
    let (container_id, inspect) = resolve_container(
        state,
        &docker,
        service_id,
        container_id,
        container_name,
        compose_project,
        compose_service,
    )
    .await?;
    let value = serde_json::to_value(inspect)?;
    let status = json_text(&value, &["State", "Status"])
        .or_else(|| json_text(&value, &["state", "status"]))
        .unwrap_or("unknown");
    let health = json_text(&value, &["State", "Health", "Status"])
        .or_else(|| json_text(&value, &["state", "health", "status"]));
    let result_status = match health {
        Some("healthy") => "up",
        Some("starting") => "warning",
        Some("unhealthy") => "down",
        Some(_) => "warning",
        None => match status {
            "running" => "up",
            "restarting" => "warning",
            _ => "down",
        },
    };
    let latency_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let error_message = (result_status == "down").then(|| {
        format!(
            "容器状态 {status}{}",
            health.map_or(String::new(), |v| format!(" / {v}"))
        )
    });
    tracing::info!(
        monitor_id = %monitor.id,
        container_id = %container_id,
        container_status = status,
        health_status = health.unwrap_or("none"),
        result = result_status,
        latency_ms,
        "Docker 容器状态检查完成"
    );
    Ok(CheckResult {
        status: result_status.into(),
        latency_ms: Some(latency_ms),
        status_code: None,
        error_message,
        extra_json: Some(
            serde_json::json!({ "container_status": status, "health_status": health }).to_string(),
        ),
    })
}

async fn resolve_container(
    state: &AppState,
    docker: &Docker,
    service_id: &str,
    old_container_id: Option<String>,
    container_name: Option<String>,
    compose_project: Option<String>,
    compose_service: Option<String>,
) -> Result<(String, Value)> {
    if let Some(container_id) = old_container_id.as_deref() {
        match docker.inspect_container(container_id, None).await {
            Ok(inspect) => return Ok((container_id.to_owned(), serde_json::to_value(inspect)?)),
            Err(error) => tracing::warn!(
                service_id,
                container_id,
                ?error,
                "Docker 容器 ID 已失效，尝试使用稳定标识重新定位"
            ),
        }
    }

    let containers = docker
        .list_containers(Some(ListContainersOptionsBuilder::new().all(true).build()))
        .await?;
    let mut fallback = None;
    for container in containers {
        let summary = serde_json::to_value(&container)?;
        if !matches_binding(
            &summary,
            container_name.as_deref(),
            compose_project.as_deref(),
            compose_service.as_deref(),
        ) {
            continue;
        }
        let Some(id) = container_id(&summary) else {
            continue;
        };
        let name = container_name_from_summary(&summary).unwrap_or_default();
        if is_running(&summary) {
            update_service_container(state, service_id, id, name).await?;
            let inspect = docker.inspect_container(id, None).await?;
            return Ok((id.to_owned(), serde_json::to_value(inspect)?));
        }
        fallback = Some((id.to_owned(), name.to_owned()));
    }

    if let Some((id, name)) = fallback {
        update_service_container(state, service_id, &id, &name).await?;
        let inspect = docker.inspect_container(&id, None).await?;
        return Ok((id, serde_json::to_value(inspect)?));
    }

    anyhow::bail!("未找到匹配的 Docker 容器，可能容器名称或 Compose 标签已变化")
}

fn matches_binding(
    summary: &Value,
    container_name: Option<&str>,
    compose_project: Option<&str>,
    compose_service: Option<&str>,
) -> bool {
    let compose_matched = compose_project
        .zip(compose_service)
        .is_some_and(|(project, service)| {
            label(summary, "com.docker.compose.project") == Some(project)
                && label(summary, "com.docker.compose.service") == Some(service)
        });
    compose_matched
        || container_name.is_some_and(|name| {
            container_name_from_summary(summary).is_some_and(|current| {
                current.trim_start_matches('/') == name.trim_start_matches('/')
            })
        })
}

async fn update_service_container(
    state: &AppState,
    service_id: &str,
    container_id: &str,
    container_name: &str,
) -> Result<()> {
    sqlx::query("UPDATE services SET docker_container_id = ?, docker_name = ? WHERE id = ?")
        .bind(container_id)
        .bind(container_name.trim_start_matches('/'))
        .bind(service_id)
        .execute(&state.pool)
        .await?;
    Ok(())
}

fn json_text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |current, key| current.get(key))
        .and_then(Value::as_str)
}

fn container_id(value: &Value) -> Option<&str> {
    value
        .get("Id")
        .or_else(|| value.get("id"))
        .and_then(Value::as_str)
}

fn container_name_from_summary(value: &Value) -> Option<&str> {
    value
        .get("Names")
        .or_else(|| value.get("names"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .map(|name| name.trim_start_matches('/'))
}

fn label<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get("Labels")
        .or_else(|| value.get("labels"))
        .and_then(Value::as_object)
        .and_then(|labels| labels.get(key))
        .and_then(Value::as_str)
}

fn is_running(value: &Value) -> bool {
    value
        .get("State")
        .or_else(|| value.get("state"))
        .and_then(Value::as_str)
        .is_some_and(|state| state == "running")
}
