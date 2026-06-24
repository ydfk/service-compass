use anyhow::Result;
use bollard::query_parameters::ListContainersOptionsBuilder;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    docker::{client, mapper},
    models::docker::{DockerCandidate, DockerEndpointRow},
    state::AppState,
};

pub async fn scan(state: &AppState, endpoint: &DockerEndpointRow) -> Result<Vec<DockerCandidate>> {
    let docker = client::connect(state, endpoint)?;
    let containers = docker
        .list_containers(Some(ListContainersOptionsBuilder::new().all(true).build()))
        .await?;
    let mut candidates = Vec::with_capacity(containers.len());
    for container in containers {
        let summary = serde_json::to_value(&container)?;
        let container_id = summary
            .get("Id")
            .or_else(|| summary.get("id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let inspect = if container_id.is_empty() {
            serde_json::Value::Null
        } else {
            docker
                .inspect_container(container_id, None)
                .await
                .map_or(serde_json::Value::Null, |value| {
                    serde_json::to_value(value).unwrap_or_default()
                })
        };
        candidates.push(mapper::candidate(endpoint, &summary, &inspect));
    }
    cache(state, endpoint, &candidates).await?;
    Ok(candidates)
}

async fn cache(
    state: &AppState,
    endpoint: &DockerEndpointRow,
    candidates: &[DockerCandidate],
) -> Result<()> {
    let mut transaction = state.pool.begin().await?;
    sqlx::query("DELETE FROM docker_scan_cache WHERE endpoint_id = ?")
        .bind(&endpoint.id)
        .execute(&mut *transaction)
        .await?;
    let scanned_at = Utc::now().to_rfc3339();
    for candidate in candidates {
        sqlx::query(
            "INSERT INTO docker_scan_cache (id, endpoint_id, container_id, container_name, image, state, status, \
             labels_json, ports_json, networks_json, candidates_json, scanned_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, NULL, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&endpoint.id)
        .bind(&candidate.container_id)
        .bind(&candidate.container_name)
        .bind(&candidate.image)
        .bind(&candidate.state)
        .bind(&candidate.status)
        .bind(serde_json::to_string(&candidate.ports)?)
        .bind(serde_json::to_string(candidate)?)
        .bind(&scanned_at)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}
