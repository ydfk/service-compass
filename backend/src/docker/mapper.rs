use std::collections::HashMap;

use serde_json::Value;

use crate::models::docker::{DockerCandidate, DockerEndpointRow};

pub fn candidate(
    endpoint: &DockerEndpointRow,
    summary: &Value,
    inspect: &Value,
) -> DockerCandidate {
    let labels = labels(summary, inspect);
    let container_id = text(summary, &["Id", "id"]);
    let container_name = summary
        .get("Names")
        .or_else(|| summary.get("names"))
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim_start_matches('/')
        .to_owned();
    let image = optional_text(summary, &["Image", "image"]);
    let compose_service = labels.get("com.docker.compose.service").cloned();
    let compose_project = labels.get("com.docker.compose.project").cloned();
    let suggested_name = compose_service
        .clone()
        .or_else(|| labels.get("homepage.name").cloned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback_name(&container_name, image.as_deref()));
    let suggested_group = labels
        .get("homepage.group")
        .cloned()
        .or_else(|| compose_project.clone())
        .unwrap_or_else(|| "Docker".into());
    let ports = ports(summary);
    let local_url = discover_url(&labels).or_else(|| port_url(endpoint, summary));
    let public_url = labels
        .get("homepage.href")
        .filter(|value| valid_url(value))
        .cloned()
        .or_else(|| {
            endpoint
                .public_host_hint
                .as_ref()
                .filter(|host| !host.trim().is_empty())
                .map(|host| normalize_host(host))
        });

    DockerCandidate {
        endpoint_id: endpoint.id.clone(),
        container_id,
        container_name,
        image,
        state: optional_text(summary, &["State", "state"]),
        status: optional_text(summary, &["Status", "status"]),
        suggested_icon: labels
            .get("homepage.icon")
            .cloned()
            .unwrap_or_else(|| slug(&suggested_name)),
        suggested_name,
        suggested_group,
        local_url,
        public_url,
        compose_project,
        compose_service,
        ports,
    }
}

fn labels(summary: &Value, inspect: &Value) -> HashMap<String, String> {
    summary
        .get("Labels")
        .or_else(|| summary.get("labels"))
        .or_else(|| inspect.pointer("/Config/Labels"))
        .or_else(|| inspect.pointer("/config/labels"))
        .and_then(Value::as_object)
        .map(|values| {
            values
                .iter()
                .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

fn ports(summary: &Value) -> Vec<String> {
    summary
        .get("Ports")
        .or_else(|| summary.get("ports"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|port| {
            let private = number(port, &["PrivatePort", "private_port"]).unwrap_or_default();
            let public = number(port, &["PublicPort", "public_port"]);
            let kind = text(port, &["Type", "typ"]);
            public.map_or_else(
                || format!("{private}/{kind}"),
                |published| format!("{published}:{private}/{kind}"),
            )
        })
        .collect()
}

fn port_url(endpoint: &DockerEndpointRow, summary: &Value) -> Option<String> {
    let host = endpoint.lan_host.as_deref()?;
    let port = summary
        .get("Ports")
        .or_else(|| summary.get("ports"))
        .and_then(Value::as_array)?
        .iter()
        .find_map(|value| number(value, &["PublicPort", "public_port"]))?;
    let scheme = if matches!(port, 443 | 8443 | 9443) {
        "https"
    } else {
        "http"
    };
    Some(format!("{scheme}://{host}:{port}"))
}

fn discover_url(labels: &HashMap<String, String>) -> Option<String> {
    labels
        .get("homepage.href")
        .cloned()
        .filter(|value| valid_url(value))
        .or_else(|| {
            labels.iter().find_map(|(key, value)| {
                key.starts_with("traefik.http.routers.")
                    .then(|| host_from_traefik(value))
                    .flatten()
            })
        })
        .or_else(|| labels.get("VIRTUAL_HOST").map(|host| normalize_host(host)))
        .or_else(|| labels.get("caddy").map(|host| normalize_host(host)))
}

fn host_from_traefik(rule: &str) -> Option<String> {
    let start = rule.find("Host(")? + 5;
    let host = rule[start..]
        .trim_end_matches(')')
        .trim_matches(['`', '\'', '"']);
    (!host.is_empty()).then(|| normalize_host(host))
}

fn normalize_host(host: &str) -> String {
    let host = host.trim();
    if host.starts_with("http://") || host.starts_with("https://") {
        host.to_owned()
    } else {
        format!("https://{host}")
    }
}

fn valid_url(value: &str) -> bool {
    let value = value.trim();
    (value.starts_with("http://") && value.len() > "http://".len())
        || (value.starts_with("https://") && value.len() > "https://".len())
}

fn fallback_name(container: &str, image: Option<&str>) -> String {
    if !container.is_empty() {
        return container.to_owned();
    }
    image
        .unwrap_or("Docker Service")
        .split('@')
        .next()
        .unwrap_or("Docker Service")
        .split(':')
        .next()
        .unwrap_or("Docker Service")
        .rsplit('/')
        .next()
        .unwrap_or("Docker Service")
        .to_owned()
}

fn slug(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn optional_text(value: &Value, keys: &[&str]) -> Option<String> {
    let value = text(value, keys);
    (!value.is_empty()).then_some(value)
}

fn text(value: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| value.get(*key)?.as_str())
        .unwrap_or_default()
        .to_owned()
}

fn number(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| value.get(*key)?.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_published_port_to_lan_url() {
        let endpoint = DockerEndpointRow {
            id: "docker".into(),
            name: "Local".into(),
            endpoint_type: "local_socket".into(),
            endpoint_url: "unix:///var/run/docker.sock".into(),
            tls_enabled: false,
            tls_ca_secret: None,
            tls_cert_secret: None,
            tls_key_secret: None,
            lan_host: Some("10.0.0.251".into()),
            public_host_hint: None,
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let summary = serde_json::json!({"Id":"abc","Names":["/plex"],"Image":"plex:latest","Ports":[{"PrivatePort":32400,"PublicPort":29000,"Type":"tcp"}]});
        let candidate = candidate(&endpoint, &summary, &Value::Null);
        assert_eq!(
            candidate.local_url.as_deref(),
            Some("http://10.0.0.251:29000")
        );
        assert_eq!(candidate.suggested_icon, "plex");
    }
}
