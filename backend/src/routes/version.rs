use std::time::Duration;

use axum::{Json, Router, routing::get};
use serde::Serialize;
use serde_json::Value;

use crate::state::AppState;

const DEFAULT_RELEASE_API: &str =
    "https://api.github.com/repos/ydfk/service-compass/releases/latest";

#[derive(Serialize)]
struct VersionInfo {
    current_version: &'static str,
    latest_version: Option<String>,
    update_available: bool,
    release_url: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/version", get(version))
}

pub fn current_version() -> &'static str {
    match option_env!("SERVICECOMPASS_VERSION") {
        Some(version) if !version.trim().is_empty() => version,
        _ => env!("CARGO_PKG_VERSION"),
    }
}

async fn version() -> Json<VersionInfo> {
    let latest = fetch_latest().await;
    let latest_version = latest.as_ref().map(|item| item.0.clone());
    let release_url = latest.map(|item| item.1);
    let update_available = latest_version
        .as_deref()
        .is_some_and(|latest| normalize(latest) != normalize(current_version()));
    Json(VersionInfo {
        current_version: current_version(),
        latest_version,
        update_available,
        release_url,
    })
}

async fn fetch_latest() -> Option<(String, String)> {
    let url =
        std::env::var("SERVICECOMPASS_RELEASE_API").unwrap_or_else(|_| DEFAULT_RELEASE_API.into());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let value: Value = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "ServiceCompass")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let tag = value.get("tag_name")?.as_str()?.to_owned();
    let html_url = value
        .get("html_url")
        .and_then(Value::as_str)
        .unwrap_or("https://github.com/ydfk/service-compass/releases")
        .to_owned();
    Some((tag, html_url))
}

fn normalize(version: &str) -> &str {
    version.trim().trim_start_matches('v')
}
