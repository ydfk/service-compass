use chrono::{Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    models::{monitor::MonitorRow, notification::NotificationEvent},
    monitor::CheckResult,
    notify,
    state::AppState,
};

#[derive(FromRow)]
struct DeliveryTarget {
    channel_id: String,
    channel_type: String,
    config_secret: String,
    cooldown_sec: i64,
    notify_on_down: bool,
    notify_on_recovery: bool,
    notify_on_warning: bool,
}

pub async fn dispatch(
    state: &AppState,
    monitor: &MonitorRow,
    previous: &str,
    result: &CheckResult,
    checked_at: &str,
) {
    let Some(event_type) = transition(previous, &result.status) else {
        return;
    };
    let targets = sqlx::query_as::<_, DeliveryTarget>(
        "SELECT c.id AS channel_id, c.channel_type, c.config_secret, r.cooldown_sec, \
         r.notify_on_down, r.notify_on_recovery, r.notify_on_warning \
         FROM notification_rules r JOIN notification_channels c ON c.id = r.channel_id \
         WHERE r.enabled = 1 AND c.enabled = 1 AND (r.monitor_id IS NULL OR r.monitor_id = ?)",
    )
    .bind(&monitor.id)
    .fetch_all(&state.pool)
    .await;
    let Ok(targets) = targets else {
        tracing::warn!(monitor_id = %monitor.id, "读取通知规则失败");
        return;
    };
    let service = load_service(state, monitor).await;
    let event = NotificationEvent {
        event_type: event_type.into(),
        monitor_id: monitor.id.clone(),
        monitor_name: monitor.name.clone(),
        service_name: service.as_ref().map(|item| item.0.clone()),
        status: result.status.clone(),
        message: result
            .error_message
            .clone()
            .unwrap_or_else(|| "服务已恢复".into()),
        target: monitor
            .target_url
            .clone()
            .or_else(|| monitor.domain.clone()),
        checked_at: checked_at.into(),
    };
    let client = reqwest::Client::new();
    for target in targets {
        if !enabled_for(&target, event_type)
            || in_cooldown(state, monitor, &target, event_type).await
        {
            continue;
        }
        let config = state
            .secrets
            .decrypt(&target.config_secret)
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok());
        let Some(config) = config else {
            tracing::warn!(channel_id = %target.channel_id, "通知配置解密失败");
            continue;
        };
        match notify::send(&client, &target.channel_type, &config, &event).await {
            Ok(_) => record_delivery(state, monitor, &target, event_type).await,
            Err(error) => tracing::warn!(channel_id = %target.channel_id, ?error, "通知发送失败"),
        }
    }
}

fn transition(previous: &str, current: &str) -> Option<&'static str> {
    match (previous, current) {
        ("up", "down") | ("warning", "down") => Some("monitor_down"),
        ("down", "up") | ("warning", "up") => Some("monitor_recovery"),
        ("up", "warning") => Some("monitor_warning"),
        _ => None,
    }
}

fn enabled_for(target: &DeliveryTarget, event_type: &str) -> bool {
    match event_type {
        "monitor_down" => target.notify_on_down,
        "monitor_recovery" => target.notify_on_recovery,
        "monitor_warning" => target.notify_on_warning,
        _ => false,
    }
}

async fn in_cooldown(
    state: &AppState,
    monitor: &MonitorRow,
    target: &DeliveryTarget,
    event_type: &str,
) -> bool {
    let cutoff = (Utc::now() - Duration::seconds(target.cooldown_sec.max(0))).to_rfc3339();
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM notification_deliveries WHERE monitor_id = ? AND channel_id = ? \
         AND event_type = ? AND delivered_at >= ?",
    )
    .bind(&monitor.id)
    .bind(&target.channel_id)
    .bind(event_type)
    .bind(cutoff)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0)
        > 0
}

async fn record_delivery(
    state: &AppState,
    monitor: &MonitorRow,
    target: &DeliveryTarget,
    event_type: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO notification_deliveries (id, monitor_id, channel_id, event_type, delivered_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&monitor.id)
    .bind(&target.channel_id)
    .bind(event_type)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await;
}

async fn load_service(state: &AppState, monitor: &MonitorRow) -> Option<(String,)> {
    let id = monitor.service_id.as_deref()?;
    sqlx::query_as("SELECT name FROM services WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::transition;

    #[test]
    fn only_relevant_status_changes_emit_events() {
        assert_eq!(transition("up", "down"), Some("monitor_down"));
        assert_eq!(transition("down", "up"), Some("monitor_recovery"));
        assert_eq!(transition("unknown", "down"), None);
        assert_eq!(transition("down", "down"), None);
    }
}
