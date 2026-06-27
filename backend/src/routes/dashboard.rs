use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    models::{
        group::{Group, Space},
        service::Service,
    },
    state::AppState,
};

#[derive(Serialize)]
struct DashboardSpace {
    #[serde(flatten)]
    space: Space,
    groups: Vec<DashboardGroup>,
}

#[derive(Clone, Serialize)]
struct DashboardGroup {
    #[serde(flatten)]
    group: Group,
    services: Vec<DashboardService>,
}

#[derive(Clone, Serialize)]
struct DashboardService {
    #[serde(flatten)]
    service: Service,
    status: &'static str,
    last_latency_ms: Option<i64>,
    last_error: Option<String>,
    icon_url: Option<String>,
    monitor_tracks: Vec<MonitorTrack>,
}

#[derive(Clone, Serialize)]
struct MonitorTrack {
    id: String,
    monitor_type: String,
    status: String,
    uptime_percent: Option<f64>,
    segments: Vec<String>,
    segment_details: Vec<TrackSegment>,
    last_checked_at: Option<String>,
    last_latency_ms: Option<i64>,
}

#[derive(Clone, Serialize)]
struct TrackSegment {
    status: String,
    checked_at: String,
    latency_ms: Option<i64>,
    status_code: Option<i64>,
    message: Option<String>,
}

type MonitorSummary = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<i64>,
    Option<String>,
);

type CheckSummary = (
    String,
    String,
    String,
    Option<i64>,
    Option<i64>,
    Option<String>,
);

#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(default = "default_range")]
    range: String,
}

fn default_range() -> String {
    "24h".into()
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/dashboard", get(dashboard))
        .route("/api/dashboard/summary", get(summary))
        .route("/api/services/{id}/history", get(service_history))
}

async fn dashboard(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let spaces = sqlx::query_as::<_, Space>("SELECT * FROM spaces ORDER BY sort_order, name")
        .fetch_all(&state.pool)
        .await?;
    let groups = sqlx::query_as::<_, Group>("SELECT * FROM groups ORDER BY sort_order, name")
        .fetch_all(&state.pool)
        .await?;
    let services = sqlx::query_as::<_, Service>(
        "SELECT * FROM services WHERE enabled = 1 ORDER BY group_id, sort_order, name",
    )
    .fetch_all(&state.pool)
    .await?;
    let monitors: Vec<MonitorSummary> = sqlx::query_as(
        "SELECT m.id, m.service_id, m.monitor_type, s.current_status, s.last_checked_at, s.last_latency_ms, s.last_error \
         FROM monitor_states s JOIN monitors m ON m.id = s.monitor_id \
         WHERE m.service_id IS NOT NULL AND m.enabled = 1 AND m.monitor_type <> 'cert'",
    )
    .fetch_all(&state.pool)
    .await?;
    let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
    let checks: Vec<CheckSummary> = sqlx::query_as(
        "SELECT monitor_id, status, checked_at, latency_ms, status_code, error_message \
         FROM monitor_checks WHERE checked_at >= ? ORDER BY checked_at",
    )
    .bind(cutoff)
    .fetch_all(&state.pool)
    .await?;
    let refresh_interval_sec = setting_i64(&state, "dashboard_refresh_interval_sec", 30).await?;

    let groups = groups
        .into_iter()
        .map(|group| DashboardGroup {
            services: services
                .iter()
                .filter(|service| service.group_id == group.id)
                .cloned()
                .map(|service| dashboard_service(service, &monitors, &checks))
                .collect(),
            group,
        })
        .collect::<Vec<_>>();
    let spaces = spaces
        .into_iter()
        .map(|space| DashboardSpace {
            groups: groups
                .iter()
                .filter(|group| group.group.space_id == space.id)
                .cloned()
                .collect(),
            space,
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::json!({
        "spaces": spaces,
        "groups": groups,
        "refresh_interval_sec": refresh_interval_sec
    })))
}

fn dashboard_service(
    service: Service,
    monitors: &[MonitorSummary],
    checks: &[CheckSummary],
) -> DashboardService {
    let related = monitors
        .iter()
        .filter(|item| item.1 == service.id)
        .collect::<Vec<_>>();
    let status = ["down", "warning", "unknown", "up"]
        .into_iter()
        .find(|candidate| related.iter().any(|item| item.3 == *candidate))
        .unwrap_or("unknown");
    let monitor_tracks = related
        .iter()
        .map(|item| monitor_track(item, checks))
        .collect();
    DashboardService {
        icon_url: service.icon_value.clone(),
        service,
        status,
        last_latency_ms: related.iter().find_map(|item| item.5),
        last_error: related.iter().find_map(|item| item.6.clone()),
        monitor_tracks,
    }
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

fn monitor_track(summary: &MonitorSummary, checks: &[CheckSummary]) -> MonitorTrack {
    let details = checks
        .iter()
        .filter(|item| item.0 == summary.0)
        .map(|item| TrackSegment {
            status: item.1.clone(),
            checked_at: item.2.clone(),
            latency_ms: item.3,
            status_code: item.4,
            message: item.5.clone(),
        })
        .collect::<Vec<_>>();
    let total = details.len();
    let up = details
        .iter()
        .filter(|item| item.status.as_str() == "up")
        .count();
    let segment_details = details
        .into_iter()
        .rev()
        .take(30)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    MonitorTrack {
        id: summary.0.clone(),
        monitor_type: summary.2.clone(),
        status: summary.3.clone(),
        uptime_percent: (total > 0).then(|| up as f64 / total as f64 * 100.0),
        segments: segment_details
            .iter()
            .map(|item| item.status.clone())
            .collect(),
        segment_details,
        last_checked_at: summary.4.clone(),
        last_latency_ms: summary.5,
    }
}

async fn summary(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let services: Vec<(String,)> = sqlx::query_as("SELECT id FROM services WHERE enabled = 1")
        .fetch_all(&state.pool)
        .await?;
    let monitors: Vec<(String, String, Option<i64>)> = sqlx::query_as(
        "SELECT m.service_id, s.current_status, s.last_latency_ms \
         FROM monitor_states s JOIN monitors m ON m.id = s.monitor_id \
         WHERE m.service_id IS NOT NULL AND m.enabled = 1 AND m.monitor_type <> 'cert'",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut by_service: HashMap<String, Vec<(String, Option<i64>)>> = HashMap::new();
    for (service_id, status, latency) in monitors {
        by_service
            .entry(service_id)
            .or_default()
            .push((status, latency));
    }
    let mut counts = HashMap::from([
        ("up", 0_i64),
        ("down", 0_i64),
        ("warning", 0_i64),
        ("unknown", 0_i64),
    ]);
    let mut latencies = Vec::new();
    for (service_id,) in &services {
        let status = service_status(by_service.get(service_id));
        *counts.entry(status).or_default() += 1;
        if let Some(items) = by_service.get(service_id) {
            latencies.extend(items.iter().filter_map(|item| item.1));
        }
    }
    let avg_latency_ms = (!latencies.is_empty())
        .then(|| latencies.iter().sum::<i64>() as f64 / latencies.len() as f64);
    let checks_24h: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM monitor_checks WHERE checked_at >= ?")
            .bind((Utc::now() - Duration::hours(24)).to_rfc3339())
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(serde_json::json!({
        "total": services.len(),
        "up": counts["up"],
        "down": counts["down"],
        "warning": counts["warning"],
        "unknown": counts["unknown"],
        "monitors": by_service.values().map(Vec::len).sum::<usize>(),
        "checks_24h": checks_24h,
        "avg_latency_ms": avg_latency_ms
    })))
}

fn service_status(items: Option<&Vec<(String, Option<i64>)>>) -> &'static str {
    let Some(items) = items else {
        return "unknown";
    };
    ["down", "warning", "unknown", "up"]
        .into_iter()
        .find(|candidate| items.iter().any(|item| item.0 == *candidate))
        .unwrap_or("unknown")
}

async fn service_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> AppResult<Json<serde_json::Value>> {
    let duration = match query.range.as_str() {
        "7d" => Duration::days(7),
        "30d" => Duration::days(30),
        _ => Duration::hours(24),
    };
    let cutoff = (Utc::now() - duration).to_rfc3339();
    let checks: Vec<(String, String, Option<i64>)> = sqlx::query_as(
        "SELECT c.checked_at, c.status, c.latency_ms FROM monitor_checks c \
         JOIN monitors m ON m.id = c.monitor_id WHERE m.service_id = ? AND m.monitor_type <> 'cert' AND c.checked_at >= ? \
         ORDER BY c.checked_at",
    )
    .bind(id)
    .bind(cutoff)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        serde_json::json!({ "range": query.range, "checks": checks }),
    ))
}
