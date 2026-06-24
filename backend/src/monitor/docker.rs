use std::time::Instant;

use anyhow::{Context, Result};

use crate::{
    docker::client,
    models::{docker::DockerEndpointRow, monitor::MonitorRow},
    monitor::CheckResult,
    state::AppState,
};

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
    let binding: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT docker_endpoint_id, docker_container_id FROM services WHERE id = ?")
            .bind(service_id)
            .fetch_optional(&state.pool)
            .await?;
    let (endpoint_id, container_id) = binding.context("服务不存在")?;
    let endpoint_id = endpoint_id.context("服务没有 Docker Endpoint 关联")?;
    let container_id = container_id.context("服务没有 Docker 容器关联")?;
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
        container_id = %container_id,
        "开始检查 Docker 容器状态"
    );
    let started = Instant::now();
    let docker = client::connect(state, &endpoint)?;
    let inspect = docker.inspect_container(&container_id, None).await?;
    let value = serde_json::to_value(inspect)?;
    let status = json_text(&value, &["State", "Status"])
        .or_else(|| json_text(&value, &["state", "status"]))
        .unwrap_or("unknown");
    let health = json_text(&value, &["State", "Health", "Status"])
        .or_else(|| json_text(&value, &["state", "health", "status"]));
    let result_status = match (status, health) {
        ("running", Some("unhealthy")) => "down",
        ("running", Some("starting")) | ("restarting", _) => "warning",
        ("running", _) => "up",
        _ => "down",
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

fn json_text<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |current, key| current.get(key))
        .and_then(serde_json::Value::as_str)
}
