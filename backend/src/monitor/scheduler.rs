use std::{sync::Arc, time::Duration};

use chrono::{Duration as ChronoDuration, Utc};
use tokio::sync::Semaphore;

use crate::{models::monitor::MonitorRow, monitor, state::AppState};

pub fn start(state: AppState) {
    tokio::spawn(async move {
        let semaphore = Arc::new(Semaphore::new(20));
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        let mut last_cleanup = Utc::now() - ChronoDuration::hours(1);
        loop {
            ticker.tick().await;
            dispatch_due(&state, Arc::clone(&semaphore)).await;
            if Utc::now() - last_cleanup >= ChronoDuration::hours(1) {
                cleanup_history(&state).await;
                last_cleanup = Utc::now();
            }
        }
    });
}

async fn dispatch_due(state: &AppState, semaphore: Arc<Semaphore>) {
    let monitors = sqlx::query_as::<_, MonitorRow>(
        "SELECT m.* FROM monitors m JOIN monitor_states s ON s.monitor_id = m.id \
         WHERE m.enabled = 1 AND (s.next_check_at IS NULL OR s.next_check_at <= ?) LIMIT 50",
    )
    .bind(Utc::now().to_rfc3339())
    .fetch_all(&state.pool)
    .await;
    let Ok(monitors) = monitors else {
        tracing::error!(error = ?monitors.err(), "读取待检查监控失败");
        return;
    };
    for monitor in monitors {
        let claimed_until =
            (Utc::now() + ChronoDuration::seconds(monitor.interval_sec.max(5))).to_rfc3339();
        if sqlx::query("UPDATE monitor_states SET next_check_at = ? WHERE monitor_id = ?")
            .bind(claimed_until)
            .bind(&monitor.id)
            .execute(&state.pool)
            .await
            .is_err()
        {
            continue;
        }
        let state = state.clone();
        let semaphore = Arc::clone(&semaphore);
        tokio::spawn(async move {
            let Ok(_permit) = semaphore.acquire_owned().await else {
                return;
            };
            let result = run(&state, &monitor).await;
            if let Err(error) = monitor::status::persist(&state, &monitor, &result).await {
                tracing::error!(monitor_id = %monitor.id, ?error, "保存检查结果失败");
            }
        });
    }
}

pub async fn run(state: &AppState, monitor: &MonitorRow) -> monitor::CheckResult {
    tracing::info!(monitor_id = %monitor.id, monitor_type = %monitor.monitor_type, "开始执行监控检查");
    let result = match monitor.monitor_type.as_str() {
        "http" | "http_keyword" => monitor::http::check(state, monitor).await,
        "dns" => monitor::dns::check(monitor).await,
        "cert" => monitor::cert::check(state, monitor).await,
        "docker" => monitor::docker::check(state, monitor).await,
        "postgres" => monitor::postgres::check(state, monitor).await,
        _ => monitor::CheckResult::down("暂不支持此监控类型"),
    };
    tracing::info!(
        monitor_id = %monitor.id,
        monitor_type = %monitor.monitor_type,
        status = %result.status,
        latency_ms = result.latency_ms,
        error = result.error_message.as_deref().unwrap_or(""),
        "监控检查完成"
    );
    result
}

async fn cleanup_history(state: &AppState) {
    match monitor::history::cleanup(state, false).await {
        Ok(deleted) if deleted > 0 => tracing::info!(deleted, "监控历史清理完成"),
        Ok(_) => {}
        Err(error) => tracing::warn!(?error, "清理监控历史失败"),
    }
}
