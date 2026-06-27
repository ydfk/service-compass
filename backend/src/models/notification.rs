use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow)]
pub struct NotificationChannelRow {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub enabled: bool,
    pub config_secret: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct NotificationChannelView {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub enabled: bool,
    pub configured: bool,
    pub config: Value,
}

#[derive(Deserialize)]
pub struct NotificationChannelInput {
    pub name: String,
    pub channel_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub config: Option<Value>,
}

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct NotificationRule {
    pub id: String,
    pub monitor_id: Option<String>,
    pub channel_id: String,
    pub notify_on_down: bool,
    pub notify_on_recovery: bool,
    pub notify_on_warning: bool,
    pub cooldown_sec: i64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct NotificationRuleInput {
    pub monitor_id: Option<String>,
    pub channel_id: String,
    #[serde(default = "default_true")]
    pub notify_on_down: bool,
    #[serde(default = "default_true")]
    pub notify_on_recovery: bool,
    #[serde(default = "default_true")]
    pub notify_on_warning: bool,
    #[serde(default = "default_cooldown")]
    pub cooldown_sec: i64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

const fn default_true() -> bool {
    true
}
const fn default_cooldown() -> i64 {
    300
}

#[derive(Clone, Debug)]
pub struct NotificationEvent {
    pub event_type: String,
    pub monitor_id: String,
    pub monitor_name: String,
    pub check_label: String,
    pub service_name: Option<String>,
    pub space_name: Option<String>,
    pub group_name: Option<String>,
    pub status: String,
    pub message: String,
    pub target: Option<String>,
    pub latency_ms: Option<i64>,
    pub status_code: Option<i64>,
    pub checked_at: String,
}

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct NotificationDeliveryView {
    pub id: String,
    pub monitor_id: Option<String>,
    pub monitor_name: Option<String>,
    pub service_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub channel_type: Option<String>,
    pub event_type: String,
    pub success: bool,
    pub request_method: Option<String>,
    pub request_url: Option<String>,
    pub request_payload: Option<String>,
    pub response_status_code: Option<i64>,
    pub response_summary: Option<String>,
    pub error_message: Option<String>,
    pub delivered_at: String,
}
