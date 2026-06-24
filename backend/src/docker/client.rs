use std::fs;

use anyhow::{Context, Result, bail};
use bollard::{API_DEFAULT_VERSION, Docker};

use crate::{models::docker::DockerEndpointRow, state::AppState};

pub fn connect(state: &AppState, endpoint: &DockerEndpointRow) -> Result<Docker> {
    tracing::info!(
        endpoint_id = %endpoint.id,
        endpoint_type = %endpoint.endpoint_type,
        endpoint_url = %endpoint.endpoint_url,
        tls = endpoint.tls_enabled,
        "连接 Docker Endpoint"
    );
    match endpoint.endpoint_type.as_str() {
        "local_socket" => Ok(Docker::connect_with_socket(
            &endpoint.endpoint_url,
            120,
            API_DEFAULT_VERSION,
        )?),
        "remote_tcp" if endpoint.tls_enabled => connect_tls(state, endpoint),
        "remote_tcp" => {
            if !is_private_tcp(&endpoint.endpoint_url) {
                bail!("非 TLS Docker API 仅允许私网地址");
            }
            Ok(Docker::connect_with_http(
                &endpoint.endpoint_url,
                120,
                API_DEFAULT_VERSION,
            )?)
        }
        _ => bail!("Docker Endpoint 类型无效"),
    }
}

fn connect_tls(state: &AppState, endpoint: &DockerEndpointRow) -> Result<Docker> {
    let directory = tempfile::tempdir()?;
    let key = decrypt_required(state, endpoint.tls_key_secret.as_deref(), "TLS Key")?;
    let cert = decrypt_required(state, endpoint.tls_cert_secret.as_deref(), "TLS Cert")?;
    let ca = decrypt_required(state, endpoint.tls_ca_secret.as_deref(), "TLS CA")?;
    let key_path = directory.path().join("key.pem");
    let cert_path = directory.path().join("cert.pem");
    let ca_path = directory.path().join("ca.pem");
    fs::write(&key_path, key)?;
    fs::write(&cert_path, cert)?;
    fs::write(&ca_path, ca)?;
    let docker = Docker::connect_with_ssl(
        &endpoint.endpoint_url,
        &key_path,
        &cert_path,
        &ca_path,
        120,
        API_DEFAULT_VERSION,
    )?;
    Ok(docker)
}

fn decrypt_required(state: &AppState, secret: Option<&str>, name: &str) -> Result<String> {
    let value = secret.with_context(|| format!("缺少 {name}"))?;
    state.secrets.decrypt(value)
}

fn is_private_tcp(url: &str) -> bool {
    let host = url
        .trim_start_matches("tcp://")
        .trim_start_matches("http://")
        .split(':')
        .next()
        .unwrap_or_default();
    host == "localhost"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("172.")
}
