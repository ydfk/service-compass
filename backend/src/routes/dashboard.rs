use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    models::{group::Group, service::Service},
    state::AppState,
};

#[derive(Serialize)]
struct DashboardGroup {
    #[serde(flatten)]
    group: Group,
    services: Vec<DashboardService>,
}

#[derive(Serialize)]
struct DashboardService {
    #[serde(flatten)]
    service: Service,
    status: &'static str,
    last_latency_ms: Option<i64>,
    last_error: Option<String>,
    icon_url: Option<String>,
    monitor_tracks: Vec<MonitorTrack>,
}

#[derive(Serialize)]
struct MonitorTrack {
    id: String,
    monitor_type: String,
    status: String,
    uptime_percent: Option<f64>,
    segments: Vec<String>,
    last_checked_at: Option<String>,
    last_latency_ms: Option<i64>,
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
         WHERE m.service_id IS NOT NULL AND m.enabled = 1",
    )
    .fetch_all(&state.pool)
    .await?;
    let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
    let checks: Vec<(String, String)> = sqlx::query_as(
        "SELECT monitor_id, status FROM monitor_checks WHERE checked_at >= ? ORDER BY checked_at",
    )
    .bind(cutoff)
    .fetch_all(&state.pool)
    .await?;

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
    Ok(Json(serde_json::json!({ "groups": groups })))
}

fn dashboard_service(
    service: Service,
    monitors: &[MonitorSummary],
    checks: &[(String, String)],
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
        icon_url: match service.icon_type.as_str() {
            "selfhst" => service.icon_value.as_deref().map(crate::icon::selfhst::url),
            _ => service.icon_value.clone(),
        },
        service,
        status,
        last_latency_ms: related.iter().find_map(|item| item.5),
        last_error: related.iter().find_map(|item| item.6.clone()),
        monitor_tracks,
    }
}

fn monitor_track(summary: &MonitorSummary, checks: &[(String, String)]) -> MonitorTrack {
    let statuses = checks
        .iter()
        .filter(|item| item.0 == summary.0)
        .map(|item| item.1.clone())
        .collect::<Vec<_>>();
    let total = statuses.len();
    let up = statuses
        .iter()
        .filter(|status| status.as_str() == "up")
        .count();
    MonitorTrack {
        id: summary.0.clone(),
        monitor_type: summary.2.clone(),
        status: summary.3.clone(),
        uptime_percent: (total > 0).then(|| up as f64 / total as f64 * 100.0),
        segments: statuses
            .into_iter()
            .rev()
            .take(30)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
        last_checked_at: summary.4.clone(),
        last_latency_ms: summary.5,
    }
}

async fn summary(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE enabled = 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({
        "total": total,
        "up": 0,
        "down": 0,
        "warning": 0,
        "unknown": total
    })))
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
         JOIN monitors m ON m.id = c.monitor_id WHERE m.service_id = ? AND c.checked_at >= ? \
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
