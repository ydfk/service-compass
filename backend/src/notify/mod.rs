pub mod bark;
pub mod dispatcher;
pub mod synology_chat;
pub mod webhook;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;

use crate::models::notification::NotificationEvent;

#[derive(Serialize)]
pub struct SendResult {
    pub status_code: u16,
    pub response_summary: String,
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

pub async fn response_result(response: reqwest::Response) -> Result<SendResult> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let summary = body.chars().take(512).collect::<String>();
    if !status.is_success() {
        bail!("通知服务返回 HTTP {}：{}", status.as_u16(), summary);
    }
    Ok(SendResult {
        status_code: status.as_u16(),
        response_summary: summary,
    })
}

pub fn required<'a>(config: &'a Value, key: &str) -> Result<&'a str> {
    config
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("缺少通知配置 {key}"))
}
