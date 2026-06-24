use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use super::monitor::MonitorInput;

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct Service {
    pub id: String,
    pub group_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon_type: String,
    pub icon_value: Option<String>,
    pub local_url: Option<String>,
    pub public_url: Option<String>,
    pub preferred_url_mode: String,
    pub docker_endpoint_id: Option<String>,
    pub docker_container_id: Option<String>,
    pub docker_name: Option<String>,
    pub docker_image: Option<String>,
    pub docker_compose_project: Option<String>,
    pub docker_compose_service: Option<String>,
    pub enabled: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct ServiceInput {
    pub group_id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_icon_type")]
    pub icon_type: String,
    pub icon_value: Option<String>,
    pub local_url: Option<String>,
    pub public_url: Option<String>,
    #[serde(default = "default_url_mode")]
    pub preferred_url_mode: String,
    pub docker_endpoint_id: Option<String>,
    pub docker_container_id: Option<String>,
    pub docker_name: Option<String>,
    pub docker_image: Option<String>,
    pub docker_compose_project: Option<String>,
    pub docker_compose_service: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub create_monitor: bool,
    #[serde(default = "default_monitor_type")]
    pub monitor_type: String,
    #[serde(default = "default_monitor_target_mode")]
    pub monitor_target_url_mode: String,
    pub monitor: Option<MonitorInput>,
}

fn default_icon_type() -> String {
    "auto".into()
}

fn default_url_mode() -> String {
    "local".into()
}

const fn default_enabled() -> bool {
    true
}

fn default_monitor_type() -> String {
    "http".into()
}

fn default_monitor_target_mode() -> String {
    "public".into()
}
