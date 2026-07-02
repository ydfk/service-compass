use chrono::{Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    models::{monitor::MonitorRow, notification::NotificationEvent},
    monitor::CheckResult,
    notify::{self, SendResult},
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
    if targets.is_empty() {
        record_delivery(
            state,
            monitor,
            None,
            event_type,
            None,
            Some("没有可用通知通道或通知规则".into()),
        )
        .await;
        return;
    }
    let service = load_service(state, monitor).await;
    let event = NotificationEvent {
        event_type: event_type.into(),
        monitor_id: monitor.id.clone(),
        monitor_name: monitor.name.clone(),
        check_label: check_label(monitor).into(),
        service_name: service.as_ref().map(|item| item.0.clone()),
        space_name: service.as_ref().and_then(|item| item.1.clone()),
        group_name: service.as_ref().and_then(|item| item.2.clone()),
        status: result.status.clone(),
        message: result
            .error_message
            .clone()
            .unwrap_or_else(|| default_message(&result.status)),
        target: monitor
            .target_url
            .clone()
            .or_else(|| monitor.domain.clone()),
        latency_ms: result.latency_ms,
        status_code: result.status_code,
        checked_at: checked_at.into(),
    };
    let client = reqwest::Client::new();
    for target in targets {
        if !enabled_for(&target, event_type) {
            record_delivery(
                state,
                monitor,
                Some(&target),
                event_type,
                None,
                Some("当前事件类型未启用通知".into()),
            )
            .await;
            continue;
        }
        if in_cooldown(state, monitor, &target, event_type).await {
            record_delivery(
                state,
                monitor,
                Some(&target),
                event_type,
                None,
                Some("通知冷却中，已跳过发送".into()),
            )
            .await;
            continue;
        }
        let config = state
            .secrets
            .decrypt(&target.config_secret)
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok());
        let Some(config) = config else {
            tracing::warn!(channel_id = %target.channel_id, "通知配置解密失败");
            record_delivery(
                state,
                monitor,
                Some(&target),
                event_type,
                None,
                Some("通知配置解密失败".into()),
            )
            .await;
            continue;
        };
        if !channel_allows_service(&config, monitor.service_id.as_deref()) {
            record_delivery(
                state,
                monitor,
                Some(&target),
                event_type,
                None,
                Some("通知通道未作用于当前服务".into()),
            )
            .await;
            continue;
        }
        match notify::send(&client, &target.channel_type, &config, &event).await {
            Ok(result) => {
                record_delivery(
                    state,
                    monitor,
                    Some(&target),
                    event_type,
                    Some(result),
                    None,
                )
                .await
            }
            Err(error) => {
                let message = error.to_string();
                let result = notify::failure_result(&error);
                tracing::warn!(channel_id = %target.channel_id, ?error, "通知发送失败");
                record_delivery(
                    state,
                    monitor,
                    Some(&target),
                    event_type,
                    result,
                    Some(message),
                )
                .await;
            }
        }
    }
}

fn default_message(status: &str) -> String {
    match status {
        "up" => "healthy",
        "warning" => "warning",
        "down" => "down",
        _ => "unknown",
    }
    .into()
}

fn transition(previous: &str, current: &str) -> Option<&'static str> {
    match (previous, current) {
        ("up", "down") | ("warning", "down") | ("unknown", "down") => Some("monitor_down"),
        ("down", "up") | ("warning", "up") => Some("monitor_recovery"),
        ("up", "warning") | ("unknown", "warning") => Some("monitor_warning"),
        _ => None,
    }
}

fn check_label(monitor: &MonitorRow) -> &'static str {
    match monitor.monitor_type.as_str() {
        "http" => "HTTP",
        "http_keyword" => "HTTP 关键字",
        "docker" => "Docker",
        "cert" => "HTTPS 证书",
        "dns" => "DNS",
        "postgres" => "PostgreSQL",
        _ => "监控检查",
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
         AND event_type = ? AND success = 1 AND delivered_at >= ?",
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
    target: Option<&DeliveryTarget>,
    event_type: &str,
    result: Option<SendResult>,
    error_message: Option<String>,
) {
    let success = result.is_some() && error_message.is_none();
    let (request_method, request_url, request_payload, response_status_code, response_summary) =
        result
            .map(|value| {
                (
                    Some(value.request_method),
                    Some(value.request_url),
                    Some(value.request_payload),
                    Some(i64::from(value.status_code)),
                    Some(value.response_summary),
                )
            })
            .unwrap_or_default();
    let _ = sqlx::query(
        "INSERT INTO notification_deliveries (id, monitor_id, channel_id, event_type, success, \
         request_method, request_url, request_payload, response_status_code, response_summary, error_message, delivered_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&monitor.id)
    .bind(target.map(|item| &item.channel_id))
    .bind(event_type)
    .bind(success)
    .bind(request_method)
    .bind(request_url)
    .bind(request_payload)
    .bind(response_status_code)
    .bind(response_summary)
    .bind(error_message)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await;
}

async fn load_service(
    state: &AppState,
    monitor: &MonitorRow,
) -> Option<(String, Option<String>, Option<String>)> {
    let id = monitor.service_id.as_deref()?;
    sqlx::query_as(
        "SELECT s.name, sp.name, g.name FROM services s \
         LEFT JOIN groups g ON g.id = s.group_id \
         LEFT JOIN spaces sp ON sp.id = g.space_id WHERE s.id = ?",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
}

fn channel_allows_service(config: &serde_json::Value, service_id: Option<&str>) -> bool {
    let Some(service_ids) = config
        .get("service_ids")
        .and_then(serde_json::Value::as_array)
        .filter(|items| !items.is_empty())
    else {
        return true;
    };
    let Some(service_id) = service_id else {
        return false;
    };
    service_ids
        .iter()
        .filter_map(serde_json::Value::as_str)
        .any(|item| item == service_id)
}

#[cfg(test)]
mod tests {
    use super::{default_message, transition};

    #[test]
    fn only_relevant_status_changes_emit_events() {
        assert_eq!(transition("up", "down"), Some("monitor_down"));
        assert_eq!(transition("down", "up"), Some("monitor_recovery"));
        assert_eq!(transition("unknown", "down"), Some("monitor_down"));
        assert_eq!(transition("unknown", "warning"), Some("monitor_warning"));
        assert_eq!(transition("unknown", "up"), None);
        assert_eq!(transition("down", "down"), None);
    }

    #[test]
    fn default_recovery_message_is_healthy() {
        assert_eq!(default_message("up"), "healthy");
    }
}
