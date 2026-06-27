use anyhow::Result;
use serde_json::Value;

use crate::{
    models::notification::NotificationEvent,
    notify::{SendResult, required, response_result, status_message},
};

pub async fn send(
    client: &reqwest::Client,
    config: &Value,
    event: &NotificationEvent,
) -> Result<SendResult> {
    let server = required(config, "server_url")?.trim_end_matches('/');
    let device_key = required(config, "device_key")?;
    let title = format!(
        "ServiceCompass: {} {}",
        event.service_name.as_deref().unwrap_or(&event.monitor_name),
        status_label(&event.status)
    );
    let request_url = format!("{server}/{device_key}");
    let payload = serde_json::json!({
        "title": title,
        "body": status_message(event),
        "group": config.get("group").and_then(Value::as_str).unwrap_or("ServiceCompass"),
        "sound": config.get("sound").and_then(Value::as_str).unwrap_or("bell")
    });
    tracing::info!(server, "请求 Bark 接口");
    let response = client.post(&request_url).json(&payload).send().await?;
    response_result(response, "POST", request_url, payload.to_string()).await
}

fn status_label(status: &str) -> &'static str {
    match status {
        "up" => "已恢复",
        "warning" => "警告",
        _ => "已离线",
    }
}
