use std::time::{Duration, Instant};

use hickory_resolver::{TokioResolver, net::runtime::TokioRuntimeProvider, proto::rr::RecordType};

use crate::{models::monitor::MonitorRow, monitor::CheckResult};

pub async fn check(monitor: &MonitorRow) -> CheckResult {
    match tokio::time::timeout(
        Duration::from_secs(monitor.timeout_sec.max(1) as u64),
        check_inner(monitor),
    )
    .await
    {
        Ok(result) => result.unwrap_or_else(|error| CheckResult::down(error.to_string())),
        Err(_) => CheckResult::down("DNS 检查超时"),
    }
}

async fn check_inner(monitor: &MonitorRow) -> anyhow::Result<CheckResult> {
    let domain = monitor
        .domain
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("域名不能为空"))?;
    let record_type = match monitor.record_type.as_str() {
        "A" => RecordType::A,
        "AAAA" => RecordType::AAAA,
        "CNAME" => RecordType::CNAME,
        _ => return Err(anyhow::anyhow!("DNS 记录类型无效")),
    };
    tracing::info!(monitor_id = %monitor.id, domain, record_type = %monitor.record_type, "开始 DNS 查询");
    let resolver = TokioResolver::builder(TokioRuntimeProvider::default())?.build()?;
    let started = Instant::now();
    let lookup = resolver.lookup(domain, record_type).await?;
    let values = lookup
        .answers()
        .iter()
        .map(|record| record.data.to_string())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(anyhow::anyhow!("未返回 DNS 记录"));
    }
    let matched = monitor.expected_value.as_ref().is_none_or(|expected| {
        values
            .iter()
            .any(|value| value.trim_end_matches('.') == expected.trim_end_matches('.'))
    });
    tracing::info!(monitor_id = %monitor.id, domain, count = values.len(), matched, "DNS 查询完成");
    Ok(CheckResult {
        status: if matched { "up" } else { "warning" }.into(),
        latency_ms: Some(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)),
        status_code: None,
        error_message: (!matched).then(|| "DNS 解析值与预期不一致".into()),
        extra_json: Some(
            serde_json::json!({
                "record_type": monitor.record_type,
                "values": values
            })
            .to_string(),
        ),
    })
}
