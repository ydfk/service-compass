use anyhow::{Result, bail};
use serde_json::Value;

use crate::{
    models::notification::NotificationEvent,
    notify::{SendResult, required},
};

pub async fn send(
    _client: &reqwest::Client,
    config: &Value,
    event: &NotificationEvent,
) -> Result<SendResult> {
    let base_url = required(config, "base_url")?.trim_end_matches('/');
    let token = required(config, "token")?;
    let mode = config
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("chatbot");
    let method = if mode == "incoming" {
        "incoming"
    } else {
        "chatbot"
    };
    let verify_tls = config
        .get("verify_tls")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(!verify_tls)
        .build()?;
    let text = format!(
        "[ServiceCompass] {}\n状态：{}\n原因：{}{}",
        event.service_name.as_deref().unwrap_or(&event.monitor_name),
        event.status,
        event.message,
        event
            .target
            .as_ref()
            .map_or_else(String::new, |target| format!("\n地址：{target}"))
    );
    let mut body = serde_json::json!({ "text": text });
    if method == "chatbot" {
        match config.get("target_type").and_then(Value::as_str) {
            Some("channel") => {
                let channel_id = config.get("channel_id").cloned().unwrap_or(Value::Null);
                body["channel_id"] = channel_id
                    .as_str()
                    .and_then(|value| value.parse::<i64>().ok())
                    .map_or(channel_id, Value::from)
            }
            Some("user") => {
                body["user_ids"] = config
                    .get("user_ids")
                    .cloned()
                    .unwrap_or_else(|| Value::Array(vec![]))
            }
            _ => {}
        }
    }
    tracing::info!(base_url, method, "请求 Synology Chat 接口");
    let response = client
        .post(format!("{base_url}/webapi/entry.cgi"))
        .query(&[
            ("api", "SYNO.Chat.External"),
            ("method", method),
            ("version", "2"),
            ("token", token),
        ])
        .form(&[("payload", body.to_string())])
        .send()
        .await?;
    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();
    validate_response(status, &response_body)?;
    let summary = response_body.chars().take(512).collect::<String>();
    Ok(SendResult {
        status_code: status.as_u16(),
        response_summary: summary,
    })
}

fn validate_response(status: reqwest::StatusCode, body: &str) -> Result<()> {
    let success = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("success").and_then(Value::as_bool));
    if !status.is_success() || success == Some(false) {
        let summary = body.chars().take(512).collect::<String>();
        bail!("Synology Chat 返回失败：{summary}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_response;

    #[test]
    fn http_ok_with_synology_error_is_failure() {
        let body = r#"{"error":{"code":800,"errors":"no target"},"success":false}"#;
        assert!(validate_response(reqwest::StatusCode::OK, body).is_err());
    }
}
