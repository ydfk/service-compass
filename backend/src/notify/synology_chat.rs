use anyhow::{Context, Result, bail};
use reqwest::Url;
use serde_json::Value;

use crate::{
    models::notification::NotificationEvent,
    notify::{SendResult, required},
};

struct Endpoint {
    url: Url,
    token: String,
    method: String,
}

pub async fn send(
    _client: &reqwest::Client,
    config: &Value,
    event: &NotificationEvent,
) -> Result<SendResult> {
    let endpoint = resolve_endpoint(config)?;
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
    let (method, body) = build_payload(config, &endpoint.method, text);
    if endpoint.method == "chatbot" && method == "incoming" {
        bail!(
            "Synology Chat Chatbot 模式必须填写至少一个数字用户 ID；频道通知请使用 Incoming Webhook"
        );
    }
    let (status, response_body) = send_request(&client, &endpoint, method, &body).await?;
    validate_response(method, status, &response_body)?;
    let summary = response_body.chars().take(512).collect::<String>();
    Ok(SendResult {
        status_code: status.as_u16(),
        response_summary: summary,
    })
}

pub fn validate_config(config: &Value) -> Result<()> {
    let endpoint = resolve_endpoint(config)?;
    if endpoint.method == "chatbot" && !has_chatbot_target(config) {
        bail!(
            "Synology Chat Chatbot 模式必须填写至少一个数字用户 ID；频道通知请使用 Incoming Webhook"
        );
    }
    Ok(())
}

async fn send_request(
    client: &reqwest::Client,
    endpoint: &Endpoint,
    method: &str,
    body: &Value,
) -> Result<(reqwest::StatusCode, String)> {
    tracing::info!(endpoint = %endpoint.url, method, "请求 Synology Chat 接口");
    let response = client
        .post(endpoint.url.clone())
        .query(&[
            ("api", "SYNO.Chat.External"),
            ("method", method),
            ("version", "2"),
            ("token", endpoint.token.as_str()),
        ])
        .form(&[("payload", body.to_string())])
        .send()
        .await?;
    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();
    Ok((status, response_body))
}

fn resolve_endpoint(config: &Value) -> Result<Endpoint> {
    let raw_url = required(config, "base_url")?;
    let parsed = Url::parse(raw_url).context("Synology Chat 地址无效")?;
    let query_method = query_value(&parsed, "method");
    let query_token = query_value(&parsed, "token");
    let mut url = if parsed.path().ends_with("/webapi/entry.cgi") {
        parsed
    } else {
        parsed.join("/webapi/entry.cgi")?
    };
    url.set_query(None);
    url.set_fragment(None);

    let token = config
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or(query_token)
        .context("Synology Chat Token 不能为空")?;
    let configured_method = config
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("incoming");
    let method = query_method.unwrap_or_else(|| configured_method.to_owned());
    let method = if method == "chatbot" {
        "chatbot".to_owned()
    } else {
        "incoming".to_owned()
    };
    Ok(Endpoint { url, token, method })
}

fn query_value(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, value)| key == name && !value.trim().is_empty())
        .map(|(_, value)| value.into_owned())
}

fn build_payload(config: &Value, requested_method: &str, text: String) -> (&'static str, Value) {
    let mut body = serde_json::json!({ "text": text });
    if requested_method != "chatbot" || !apply_chatbot_target(config, &mut body) {
        return ("incoming", body);
    }
    ("chatbot", body)
}

fn has_chatbot_target(config: &Value) -> bool {
    let mut body = serde_json::json!({});
    apply_chatbot_target(config, &mut body)
}

fn apply_chatbot_target(config: &Value, body: &mut Value) -> bool {
    match config.get("target_type").and_then(Value::as_str) {
        Some("user") => {
            let Some(user_ids) = config
                .get("user_ids")
                .and_then(Value::as_array)
                .filter(|values| !values.is_empty())
            else {
                return false;
            };
            let user_ids = user_ids
                .iter()
                .filter_map(positive_user_id)
                .collect::<Vec<_>>();
            if user_ids.is_empty() {
                return false;
            }
            body["user_ids"] = Value::Array(user_ids);
            true
        }
        _ => false,
    }
}

fn synology_error_code(body: &str) -> Option<i64> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("error")?
        .get("code")?
        .as_i64()
}

fn positive_user_id(value: &Value) -> Option<Value> {
    let id = match value {
        Value::Number(number) => number.as_i64()?,
        Value::String(text) => text.trim().parse::<i64>().ok()?,
        _ => return None,
    };
    (id > 0).then_some(Value::Number(id.into()))
}

fn validate_response(method: &str, status: reqwest::StatusCode, body: &str) -> Result<()> {
    let success = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("success").and_then(Value::as_bool));
    if !status.is_success() || success == Some(false) {
        let summary = body.chars().take(512).collect::<String>();
        if method == "chatbot" && matches!(synology_error_code(body), Some(117) | Some(800)) {
            bail!(
                "Synology Chat Chatbot 返回失败：{summary}。Chatbot 只能发送给 Bot 可见的数字 user_id；如果要发到频道，请在 Synology Chat 创建 Incoming Webhook 并选择 Incoming 模式"
            );
        }
        if method == "incoming" && matches!(synology_error_code(body), Some(404)) {
            bail!(
                "Synology Chat Incoming Webhook 返回失败：{summary}。当前 Token 可能是 Chatbot Token；请使用 Incoming Webhook URL，或切换到 Chatbot 模式并填写数字用户 ID"
            );
        }
        bail!("Synology Chat 返回失败：{summary}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_payload, resolve_endpoint, validate_config, validate_response};

    #[test]
    fn full_webhook_url_supplies_method_and_token() {
        let endpoint = resolve_endpoint(&json!({
            "base_url": "http://nas:5000/webapi/entry.cgi?api=SYNO.Chat.External&method=incoming&version=2&token=secret"
        }))
        .unwrap();
        assert_eq!(endpoint.url.as_str(), "http://nas:5000/webapi/entry.cgi");
        assert_eq!(endpoint.method, "incoming");
        assert_eq!(endpoint.token, "secret");
    }

    #[test]
    fn full_url_method_takes_precedence_over_selected_mode() {
        let endpoint = resolve_endpoint(&json!({
            "mode": "incoming",
            "base_url": "http://nas:5000/webapi/entry.cgi?api=SYNO.Chat.External&method=chatbot&version=2&token=secret"
        }))
        .unwrap();
        assert_eq!(endpoint.method, "chatbot");
    }

    #[test]
    fn chatbot_channel_target_is_not_supported() {
        let config = json!({ "target_type": "channel", "channel_id": "42" });
        let (method, payload) = build_payload(&config, "chatbot", "hello".into());
        assert_eq!(method, "incoming");
        assert_eq!(payload["text"], "hello");
    }

    #[test]
    fn chatbot_user_target_uses_numeric_ids() {
        let config = json!({ "target_type": "user", "user_ids": ["12", 15] });
        let (method, payload) = build_payload(&config, "chatbot", "hello".into());
        assert_eq!(method, "chatbot");
        assert_eq!(payload["user_ids"], json!([12, 15]));
    }

    #[test]
    fn chatbot_without_target_uses_incoming_compatibility() {
        let (method, payload) = build_payload(&json!({}), "chatbot", "hello".into());
        assert_eq!(method, "incoming");
        assert_eq!(payload["text"], "hello");
    }

    #[test]
    fn new_chatbot_config_requires_target() {
        let config = json!({
            "base_url": "http://nas:5000",
            "token": "secret",
            "mode": "chatbot"
        });
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn http_ok_with_synology_error_is_failure() {
        let body = r#"{"error":{"code":800,"errors":"no target"},"success":false}"#;
        assert!(validate_response("chatbot", reqwest::StatusCode::OK, body).is_err());
    }
}
