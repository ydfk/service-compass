use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use x509_parser::{extensions::GeneralName, parse_x509_certificate};

use crate::{models::monitor::MonitorRow, monitor::CheckResult};

pub async fn check(monitor: &MonitorRow) -> CheckResult {
    match tokio::time::timeout(
        Duration::from_secs(monitor.timeout_sec.max(1) as u64),
        check_inner(monitor),
    )
    .await
    {
        Ok(result) => result.unwrap_or_else(|error| CheckResult::down(error.to_string())),
        Err(_) => CheckResult::down("证书检查超时"),
    }
}

async fn check_inner(monitor: &MonitorRow) -> anyhow::Result<CheckResult> {
    let domain = monitor
        .domain
        .as_deref()
        .filter(|value| !value.is_empty())
        .context("域名不能为空")?;
    let port = u16::try_from(monitor.cert_port).context("证书端口无效")?;
    tracing::info!(monitor_id = %monitor.id, domain, port, "开始 TLS 证书检查");
    let started = Instant::now();
    let stream = TcpStream::connect((domain, port)).await?;
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let server_name = ServerName::try_from(domain.to_owned()).context("域名无效")?;
    let tls = connector.connect(server_name, stream).await?;
    let certificates = tls
        .get_ref()
        .1
        .peer_certificates()
        .context("服务端没有返回证书")?;
    let certificate = certificates.first().context("证书链为空")?;
    let (_, parsed) = parse_x509_certificate(certificate.as_ref())?;
    let not_after = parsed.validity().not_after.timestamp();
    let now = chrono::Utc::now().timestamp();
    let days_left = (not_after - now) / 86_400;
    let status = if days_left <= monitor.cert_critical_days {
        "down"
    } else if days_left <= monitor.cert_warning_days {
        "warning"
    } else {
        "up"
    };
    tracing::info!(monitor_id = %monitor.id, domain, port, days_left, status, "TLS 证书检查完成");
    let sans = parsed
        .subject_alternative_name()?
        .map(|extension| {
            extension
                .value
                .general_names
                .iter()
                .filter_map(|name| match name {
                    GeneralName::DNSName(value) => Some((*value).to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(CheckResult {
        status: status.into(),
        latency_ms: Some(i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX)),
        status_code: None,
        error_message: (status != "up").then(|| format!("证书剩余 {days_left} 天")),
        extra_json: Some(
            serde_json::json!({
                "issuer": parsed.issuer().to_string(),
                "subject": parsed.subject().to_string(),
                "not_before": parsed.validity().not_before.to_rfc2822().ok(),
                "not_after": parsed.validity().not_after.to_rfc2822().ok(),
                "days_left": days_left,
                "sans": sans
            })
            .to_string(),
        ),
    })
}
