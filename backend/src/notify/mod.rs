pub mod bark;
pub mod dispatcher;
pub mod synology_chat;
pub mod webhook;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local};
use serde::Serialize;
use serde_json::Value;
use std::{error::Error, fmt};

use crate::models::notification::NotificationEvent;

#[derive(Clone, Debug, Serialize)]
pub struct SendResult {
    pub status_code: u16,
    pub response_summary: String,
    pub request_method: String,
    pub request_url: String,
    pub request_payload: String,
}

#[derive(Debug)]
pub struct SendFailure {
    pub result: SendResult,
    pub message: String,
}

impl fmt::Display for SendFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SendFailure {}

impl SendResult {
    pub fn empty() -> Self {
        Self {
            status_code: 0,
            response_summary: String::new(),
            request_method: String::new(),
            request_url: String::new(),
            request_payload: String::new(),
        }
    }
}

pub fn failure_result(error: &anyhow::Error) -> Option<SendResult> {
    error
        .downcast_ref::<SendFailure>()
        .map(|failure| failure.result.clone())
}

pub async fn send(
    client: &reqwest::Client,
    channel_type: &str,
    config: &Value,
    event: &NotificationEvent,
) -> Result<SendResult> {
    tracing::info!(
        channel_type,
        monitor_id = %event.monitor_id,
        event_type = %event.event_type,
        "开始发送第三方通知"
    );
    let result = match channel_type {
        "bark" => bark::send(client, config, event).await,
        "webhook" => webhook::send(client, config, event).await,
        "synology_chat" => synology_chat::send(client, config, event).await,
        _ => bail!("不支持的通知类型"),
    };
    match &result {
        Ok(value) => tracing::info!(
            channel_type,
            status_code = value.status_code,
            "第三方通知发送完成"
        ),
        Err(error) => tracing::warn!(channel_type, ?error, "第三方通知发送失败"),
    }
    result
}

pub async fn response_result(
    response: reqwest::Response,
    request_method: impl Into<String>,
    request_url: impl Into<String>,
    request_payload: impl Into<String>,
) -> Result<SendResult> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let summary = body.chars().take(512).collect::<String>();
    let result = SendResult {
        status_code: status.as_u16(),
        response_summary: summary.clone(),
        request_method: request_method.into(),
        request_url: request_url.into(),
        request_payload: request_payload.into().chars().take(2048).collect(),
    };
    if !status.is_success() {
        return Err(SendFailure {
            result,
            message: format!("通知服务返回 HTTP {}：{}", status.as_u16(), summary),
        }
        .into());
    }
    Ok(result)
}

pub fn required<'a>(config: &'a Value, key: &str) -> Result<&'a str> {
    config
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("缺少通知配置 {key}"))
}

pub fn status_icon(status: &str) -> &'static str {
    match status {
        "up" => "✅ Up",
        "warning" => "🟡 Warning",
        "down" => "🔴 Down",
        _ => "⚪ Unknown",
    }
}

pub fn status_message(event: &NotificationEvent) -> String {
    let service = event.service_name.as_deref().unwrap_or(&event.monitor_name);
    let target = event.target.as_deref().unwrap_or("");
    let status = status_icon(&event.status);
    let latency = event
        .latency_ms
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| "—".into());
    let checked_at = format_checked_at(&event.checked_at);
    let detail = detail_text(event);
    let location = location_line(event);
    let status_code = event
        .status_code
        .map(|value| format!("\n状态码：{value}"))
        .unwrap_or_default();

    format!(
        "{status} · {service} · {check_label}\n{detail}\n\n服务：{service}{location}\n检查项：{check_label}\n监控名称：{monitor}\n地址：{target}\n状态：{status}\n响应时间：{latency}\n检查时间：{checked_at}{status_code}\n详情：{detail}\n\n—— ServiceCompass ——",
        check_label = event.check_label,
        monitor = event.monitor_name
    )
}

fn location_line(event: &NotificationEvent) -> String {
    let Some(space) = event.space_name.as_deref() else {
        return String::new();
    };
    let group = event.group_name.as_deref().unwrap_or("未分组");
    format!("\n位置：{space} / {group}")
}

fn detail_text(event: &NotificationEvent) -> String {
    if !event.message.trim().is_empty() {
        return event.message.clone();
    }
    match event.status.as_str() {
        "up" => "healthy".into(),
        "warning" => "warning".into(),
        "down" => "down".into(),
        _ => "unknown".into(),
    }
}

fn format_checked_at(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|time| {
            time.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S%.3f")
                .to_string()
        })
        .unwrap_or_else(|_| value.to_owned())
}

#[cfg(test)]
mod tests {
    use crate::{models::notification::NotificationEvent, notify::status_message};

    #[test]
    fn status_message_matches_chat_style() {
        let event = NotificationEvent {
            event_type: "monitor_down".into(),
            monitor_id: "monitor-1".into(),
            monitor_name: "HTTP".into(),
            check_label: "HTTP".into(),
            service_name: Some("new-api".into()),
            space_name: Some("Domestic".into()),
            group_name: Some("home".into()),
            status: "down".into(),
            message: "Request failed with status code 404".into(),
            target: Some("https://example.com".into()),
            latency_ms: Some(123),
            status_code: Some(404),
            checked_at: "2026-06-24T19:01:00.170+08:00".into(),
        };

        let message = status_message(&event);
        assert!(
            message.starts_with("🔴 Down · new-api · HTTP\nRequest failed with status code 404")
        );
        assert!(message.contains("服务：new-api"));
        assert!(message.contains("位置：Domestic / home"));
        assert!(message.contains("检查项：HTTP"));
        assert!(message.contains("监控名称：HTTP"));
        assert!(message.contains("状态：🔴 Down"));
        assert!(message.contains("响应时间：123 ms"));
        assert!(message.contains("状态码：404"));
        assert!(message.contains("详情：Request failed with status code 404"));
        assert!(message.ends_with("—— ServiceCompass ——"));
    }
}
