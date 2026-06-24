use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use reqwest::{Client, Method, redirect::Policy};

use crate::{models::monitor::MonitorRow, monitor::CheckResult, state::AppState};

pub async fn check(state: &AppState, monitor: &MonitorRow) -> CheckResult {
    let attempts = monitor.retries.max(0) + 1;
    let mut last = CheckResult::down("检查未执行");
    for attempt in 0..attempts {
        last = match check_once(state, monitor).await {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(
                    monitor_id = %monitor.id,
                    attempt = attempt + 1,
                    attempts,
                    ?error,
                    "HTTP 监控请求失败"
                );
                CheckResult::down(error.to_string())
            }
        };
        if last.status == "up" || attempt + 1 == attempts {
            return last;
        }
        tokio::time::sleep(Duration::from_secs(monitor.retry_interval_sec.max(0) as u64)).await;
    }
    last
}

async fn check_once(state: &AppState, monitor: &MonitorRow) -> Result<CheckResult> {
    let url = resolve_url(state, monitor).await?;
    tracing::info!(
        monitor_id = %monitor.id,
        method = %monitor.method,
        url = %url,
        "开始 HTTP 监控请求"
    );
    let client = Client::builder()
        .timeout(Duration::from_secs(monitor.timeout_sec.max(1) as u64))
        .danger_accept_invalid_certs(!monitor.tls_verify)
        .redirect(if monitor.follow_redirects {
            Policy::limited(10)
        } else {
            Policy::none()
        })
        .build()?;
    let method = Method::from_bytes(monitor.method.as_bytes()).context("请求方法无效")?;
    let mut request = client.request(method, &url);
    if monitor.auth_type == "basic" {
        let username = monitor.auth_username.as_deref().unwrap_or_default();
        let password = monitor
            .auth_password_secret
            .as_deref()
            .map(|secret| state.secrets.decrypt(secret))
            .transpose()?
            .unwrap_or_default();
        request = request.basic_auth(username, Some(password));
    }

    let started = Instant::now();
    let response = request.send().await?;
    let latency_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let status_code = i64::from(response.status().as_u16());
    tracing::info!(
        monitor_id = %monitor.id,
        status_code,
        latency_ms,
        "HTTP 监控收到响应"
    );
    if !(monitor.expected_status_min..=monitor.expected_status_max).contains(&status_code) {
        bail!("HTTP 状态码 {status_code} 不在预期范围");
    }
    if monitor.monitor_type == "http_keyword" {
        let keyword = monitor.keyword.as_deref().context("关键字不能为空")?;
        let body = response.text().await?;
        if !body.contains(keyword) {
            bail!("响应内容不包含关键字");
        }
    }
    Ok(CheckResult {
        status: "up".into(),
        latency_ms: Some(latency_ms),
        status_code: Some(status_code),
        error_message: None,
        extra_json: None,
    })
}

async fn resolve_url(state: &AppState, monitor: &MonitorRow) -> Result<String> {
    if monitor.target_url_mode == "custom" {
        return monitor.target_url.clone().context("目标 URL 不能为空");
    }
    let service_id = monitor.service_id.as_deref().context("监控未关联服务")?;
    let row: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT local_url, public_url FROM services WHERE id = ?")
            .bind(service_id)
            .fetch_optional(&state.pool)
            .await?;
    let (local, public) = row.context("关联服务不存在")?;
    match monitor.target_url_mode.as_str() {
        "local" => local.or(public),
        "public" => public.or(local),
        _ => None,
    }
    .context("关联服务没有可用地址")
}
