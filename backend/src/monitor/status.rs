use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{error::AppResult, models::monitor::MonitorRow, monitor::CheckResult, state::AppState};

pub async fn persist(
    state: &AppState,
    monitor: &MonitorRow,
    result: &CheckResult,
) -> AppResult<()> {
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let next = (now + Duration::seconds(monitor.interval_sec.max(5))).to_rfc3339();
    let previous: Option<(String, i64)> = sqlx::query_as(
        "SELECT current_status, consecutive_failures FROM monitor_states WHERE monitor_id = ?",
    )
    .bind(&monitor.id)
    .fetch_optional(&state.pool)
    .await?;
    let previous_status = previous
        .as_ref()
        .map(|item| item.0.clone())
        .unwrap_or_else(|| "unknown".into());
    let failures = if result.status == "down" {
        previous.as_ref().map_or(1, |item| item.1 + 1)
    } else {
        0
    };

    sqlx::query(
        "INSERT INTO monitor_checks (id, monitor_id, status, latency_ms, status_code, error_message, checked_at, extra_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&monitor.id)
    .bind(&result.status)
    .bind(result.latency_ms)
    .bind(result.status_code)
    .bind(&result.error_message)
    .bind(&now_text)
    .bind(&result.extra_json)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO monitor_states (monitor_id, current_status, previous_status, consecutive_failures, last_checked_at, \
         last_up_at, last_down_at, last_latency_ms, last_error, next_check_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(monitor_id) DO UPDATE SET current_status = excluded.current_status, \
         previous_status = monitor_states.current_status, consecutive_failures = excluded.consecutive_failures, \
         last_checked_at = excluded.last_checked_at, \
         last_up_at = COALESCE(excluded.last_up_at, monitor_states.last_up_at), \
         last_down_at = COALESCE(excluded.last_down_at, monitor_states.last_down_at), \
         last_latency_ms = excluded.last_latency_ms, last_error = excluded.last_error, \
         next_check_at = excluded.next_check_at, updated_at = excluded.updated_at",
    )
    .bind(&monitor.id)
    .bind(&result.status)
    .bind(&previous_status)
    .bind(failures)
    .bind(&now_text)
    .bind((result.status == "up").then_some(&now_text))
    .bind((result.status == "down").then_some(&now_text))
    .bind(result.latency_ms)
    .bind(&result.error_message)
    .bind(next)
    .bind(&now_text)
    .execute(&state.pool)
    .await?;
    crate::notify::dispatcher::dispatch(state, monitor, &previous_status, result, &now_text).await;
    let _ = state.dashboard_events.send(now_text.clone());
    tracing::info!(
        monitor_id = %monitor.id,
        previous_status,
        current_status = %result.status,
        "监控状态与历史已保存"
    );
    Ok(())
}
