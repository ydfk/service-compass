use anyhow::{Context, Result};
use reqwest::{
    Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
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
    let url = required(config, "url")?;
    let method = config
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("POST")
        .parse::<Method>()?;
    let mut headers = HeaderMap::new();
    if let Some(values) = config.get("headers").and_then(Value::as_object) {
        for (name, value) in values {
            headers.insert(
                name.parse::<HeaderName>()?,
                HeaderValue::from_str(value.as_str().context("Webhook Header 必须是文本")?)?,
            );
        }
    }
    tracing::info!(url, method = %method, "请求 Webhook 接口");
    let response = client
        .request(method, url)
        .headers(headers)
        .json(&serde_json::json!({
            "app": "ServiceCompass",
            "event": event.event_type,
            "service_name": event.service_name,
            "monitor_name": event.monitor_name,
            "status": event.status,
            "message": event.message,
            "formatted_message": status_message(event),
            "target": event.target,
            "latency_ms": event.latency_ms,
            "status_code": event.status_code,
            "checked_at": event.checked_at
        }))
        .send()
        .await?;
    response_result(response).await
}
