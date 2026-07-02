use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::json;
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgSslMode},
};

use crate::{models::monitor::MonitorRow, monitor::CheckResult, state::AppState};

const DEFAULT_QUERY: &str = "SELECT 1";

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
                    "PostgreSQL 监控检查失败"
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
    let host = monitor
        .target_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("PostgreSQL 主机不能为空")?;
    let database = monitor
        .domain
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("PostgreSQL 数据库名不能为空")?;
    let username = monitor
        .auth_username
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("PostgreSQL 用户名不能为空")?;
    let password = monitor
        .auth_password_secret
        .as_deref()
        .map(|secret| state.secrets.decrypt(secret))
        .transpose()?
        .unwrap_or_default();
    let port = u16::try_from(monitor.cert_port).context("PostgreSQL 端口无效")?;
    let query = query_text(monitor);
    tracing::info!(
        monitor_id = %monitor.id,
        host,
        port,
        database,
        "开始 PostgreSQL 监控检查"
    );

    let options = PgConnectOptions::new()
        .host(host)
        .port(port)
        .database(database)
        .username(username)
        .password(&password)
        .ssl_mode(PgSslMode::Prefer);
    let timeout = Duration::from_secs(monitor.timeout_sec.max(1) as u64);
    let started = Instant::now();
    let result = tokio::time::timeout(timeout, async {
        let mut connection = options.connect().await?;
        sqlx::query(query).fetch_optional(&mut connection).await?;
        Ok::<(), sqlx::Error>(())
    })
    .await
    .context("PostgreSQL 检查超时")?;
    result?;
    let latency_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    Ok(CheckResult {
        status: "up".into(),
        latency_ms: Some(latency_ms),
        status_code: None,
        error_message: None,
        extra_json: Some(
            json!({
                "database": database,
                "query": query,
            })
            .to_string(),
        ),
    })
}

pub fn query_text(monitor: &MonitorRow) -> &str {
    monitor
        .expected_value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_QUERY)
}
