use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow)]
pub struct DockerEndpointRow {
    pub id: String,
    pub name: String,
    pub endpoint_type: String,
    pub endpoint_url: String,
    pub tls_enabled: bool,
    pub tls_ca_secret: Option<String>,
    pub tls_cert_secret: Option<String>,
    pub tls_key_secret: Option<String>,
    pub lan_host: Option<String>,
    pub public_host_hint: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct DockerEndpointView {
    pub id: String,
    pub name: String,
    pub endpoint_type: String,
    pub endpoint_url: String,
    pub tls_enabled: bool,
    pub has_tls_ca: bool,
    pub has_tls_cert: bool,
    pub has_tls_key: bool,
    pub lan_host: Option<String>,
    pub public_host_hint: Option<String>,
    pub enabled: bool,
}

impl From<DockerEndpointRow> for DockerEndpointView {
    fn from(row: DockerEndpointRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            endpoint_type: row.endpoint_type,
            endpoint_url: row.endpoint_url,
            tls_enabled: row.tls_enabled,
            has_tls_ca: row.tls_ca_secret.is_some(),
            has_tls_cert: row.tls_cert_secret.is_some(),
            has_tls_key: row.tls_key_secret.is_some(),
            lan_host: row.lan_host,
            public_host_hint: row.public_host_hint,
            enabled: row.enabled,
        }
    }
}

#[derive(Deserialize)]
pub struct DockerEndpointInput {
    pub name: String,
    pub endpoint_type: String,
    pub endpoint_url: String,
    #[serde(default)]
    pub tls_enabled: bool,
    pub tls_ca: Option<String>,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
    pub lan_host: Option<String>,
    pub public_host_hint: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

const fn default_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DockerCandidate {
    pub endpoint_id: String,
    pub container_id: String,
    pub container_name: String,
    pub image: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub suggested_name: String,
    pub suggested_group: String,
    pub suggested_icon: String,
    pub local_url: Option<String>,
    pub public_url: Option<String>,
    pub compose_project: Option<String>,
    pub compose_service: Option<String>,
    pub ports: Vec<String>,
}

#[derive(Deserialize)]
pub struct AddCandidateInput {
    pub endpoint_id: String,
    pub container_id: String,
    pub group_id: String,
    pub name: Option<String>,
    pub local_url: Option<String>,
    pub public_url: Option<String>,
    pub icon_value: Option<String>,
    #[serde(default = "default_enabled")]
    pub create_monitor: bool,
}
