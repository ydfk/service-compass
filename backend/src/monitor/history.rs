use chrono::{Duration as ChronoDuration, Utc};

use crate::{error::AppResult, state::AppState};

pub async fn cleanup(state: &AppState, reclaim_space: bool) -> AppResult<u64> {
    let retention_days = setting_i64(state, "retention_days", 30).await?;
    let max_per_monitor = setting_i64(state, "monitor_checks_max_per_monitor", 2000).await?;
    let cutoff = (Utc::now() - ChronoDuration::days(retention_days.max(1))).to_rfc3339();

    let mut deleted = sqlx::query("DELETE FROM monitor_checks WHERE checked_at < ?")
        .bind(cutoff)
        .execute(&state.pool)
        .await?
        .rows_affected();

    deleted += sqlx::query(
        "DELETE FROM monitor_checks WHERE id IN ( \
            SELECT id FROM ( \
                SELECT id, ROW_NUMBER() OVER (PARTITION BY monitor_id ORDER BY checked_at DESC, id DESC) AS row_num \
                FROM monitor_checks \
            ) WHERE row_num > ? \
        )",
    )
    .bind(max_per_monitor.max(100))
    .execute(&state.pool)
    .await?
    .rows_affected();

    if deleted > 0 {
        sqlx::query("PRAGMA optimize").execute(&state.pool).await?;
        if reclaim_space {
            sqlx::query("VACUUM").execute(&state.pool).await?;
        }
    }
    Ok(deleted)
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
