use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow)]
pub struct MonitorRow {
    pub id: String,
    pub service_id: Option<String>,
    pub name: String,
    pub monitor_type: String,
    pub target_url: Option<String>,
    pub target_url_mode: String,
    pub method: String,
    pub expected_status_min: i64,
    pub expected_status_max: i64,
    pub keyword: Option<String>,
    pub interval_sec: i64,
    pub timeout_sec: i64,
    pub retries: i64,
    pub retry_interval_sec: i64,
    pub follow_redirects: bool,
    pub tls_verify: bool,
    pub auth_type: String,
    pub auth_username: Option<String>,
    pub auth_password_secret: Option<String>,
    pub domain: Option<String>,
    pub record_type: String,
    pub expected_value: Option<String>,
    pub cert_port: i64,
    pub cert_warning_days: i64,
    pub cert_critical_days: i64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct MonitorView {
    pub id: String,
    pub service_id: Option<String>,
    pub name: String,
    pub monitor_type: String,
    pub target_url: Option<String>,
    pub target_url_mode: String,
    pub method: String,
    pub expected_status_min: i64,
    pub expected_status_max: i64,
    pub keyword: Option<String>,
    pub interval_sec: i64,
    pub timeout_sec: i64,
    pub retries: i64,
    pub retry_interval_sec: i64,
    pub follow_redirects: bool,
    pub tls_verify: bool,
    pub auth_type: String,
    pub auth_username: Option<String>,
    pub has_auth_password: bool,
    pub domain: Option<String>,
    pub record_type: String,
    pub expected_value: Option<String>,
    pub cert_port: i64,
    pub cert_warning_days: i64,
    pub cert_critical_days: i64,
    pub enabled: bool,
    pub current_status: String,
    pub last_checked_at: Option<String>,
    pub last_latency_ms: Option<i64>,
    pub last_error: Option<String>,
    pub last_extra_json: Option<String>,
    pub recent_statuses: Vec<String>,
    pub recent_checks: Vec<MonitorCheck>,
    pub notify_enabled: bool,
    pub notification_channel_ids: Vec<String>,
    pub notify_on_down: bool,
    pub notify_on_recovery: bool,
    pub notify_on_warning: bool,
    pub notification_cooldown_sec: i64,
}

impl MonitorView {
    pub fn from_row(row: MonitorRow) -> Self {
        Self {
            id: row.id,
            service_id: row.service_id,
            name: row.name,
            monitor_type: row.monitor_type,
            target_url: row.target_url,
            target_url_mode: row.target_url_mode,
            method: row.method,
            expected_status_min: row.expected_status_min,
            expected_status_max: row.expected_status_max,
            keyword: row.keyword,
            interval_sec: row.interval_sec,
            timeout_sec: row.timeout_sec,
            retries: row.retries,
            retry_interval_sec: row.retry_interval_sec,
            follow_redirects: row.follow_redirects,
            tls_verify: row.tls_verify,
            auth_type: row.auth_type,
            auth_username: row.auth_username,
            has_auth_password: row.auth_password_secret.is_some(),
            domain: row.domain,
            record_type: row.record_type,
            expected_value: row.expected_value,
            cert_port: row.cert_port,
            cert_warning_days: row.cert_warning_days,
            cert_critical_days: row.cert_critical_days,
            enabled: row.enabled,
            current_status: "unknown".into(),
            last_checked_at: None,
            last_latency_ms: None,
            last_error: None,
            last_extra_json: None,
            recent_statuses: Vec::new(),
            recent_checks: Vec::new(),
            notify_enabled: false,
            notification_channel_ids: Vec::new(),
            notify_on_down: true,
            notify_on_recovery: true,
            notify_on_warning: true,
            notification_cooldown_sec: 300,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MonitorInput {
    pub service_id: Option<String>,
    pub name: String,
    pub monitor_type: String,
    pub target_url: Option<String>,
    #[serde(default = "default_target_mode")]
    pub target_url_mode: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_status_min")]
    pub expected_status_min: i64,
    #[serde(default = "default_status_max")]
    pub expected_status_max: i64,
    pub keyword: Option<String>,
    #[serde(default = "default_interval")]
    pub interval_sec: i64,
    #[serde(default = "default_timeout")]
    pub timeout_sec: i64,
    #[serde(default = "default_retries")]
    pub retries: i64,
    #[serde(default = "default_retry_interval")]
    pub retry_interval_sec: i64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_true")]
    pub tls_verify: bool,
    #[serde(default = "default_auth_type")]
    pub auth_type: String,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
    pub domain: Option<String>,
    #[serde(default = "default_record_type")]
    pub record_type: String,
    pub expected_value: Option<String>,
    #[serde(default = "default_cert_port")]
    pub cert_port: i64,
    #[serde(default = "default_warning_days")]
    pub cert_warning_days: i64,
    #[serde(default = "default_critical_days")]
    pub cert_critical_days: i64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub notify_enabled: bool,
    #[serde(default)]
    pub notification_channel_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub notify_on_down: bool,
    #[serde(default = "default_true")]
    pub notify_on_recovery: bool,
    #[serde(default = "default_true")]
    pub notify_on_warning: bool,
    #[serde(default = "default_cooldown")]
    pub notification_cooldown_sec: i64,
}

impl MonitorInput {
    pub fn service_http(
        service_id: String,
        name: String,
        monitor_type: String,
        target_url_mode: String,
    ) -> Self {
        Self {
            service_id: Some(service_id),
            name,
            monitor_type,
            target_url: None,
            target_url_mode,
            method: default_method(),
            expected_status_min: default_status_min(),
            expected_status_max: default_status_max(),
            keyword: None,
            interval_sec: default_interval(),
            timeout_sec: default_timeout(),
            retries: default_retries(),
            retry_interval_sec: default_retry_interval(),
            follow_redirects: true,
            tls_verify: true,
            auth_type: default_auth_type(),
            auth_username: None,
            auth_password: None,
            domain: None,
            record_type: default_record_type(),
            expected_value: None,
            cert_port: default_cert_port(),
            cert_warning_days: default_warning_days(),
            cert_critical_days: default_critical_days(),
            enabled: true,
            notify_enabled: false,
            notification_channel_ids: Vec::new(),
            notify_on_down: true,
            notify_on_recovery: true,
            notify_on_warning: true,
            notification_cooldown_sec: default_cooldown(),
        }
    }
}

const fn default_true() -> bool {
    true
}
fn default_target_mode() -> String {
    "custom".into()
}
fn default_method() -> String {
    "GET".into()
}
const fn default_status_min() -> i64 {
    200
}
const fn default_status_max() -> i64 {
    399
}
const fn default_interval() -> i64 {
    60
}
const fn default_timeout() -> i64 {
    10
}
const fn default_retries() -> i64 {
    2
}
const fn default_retry_interval() -> i64 {
    5
}
fn default_auth_type() -> String {
    "none".into()
}
fn default_record_type() -> String {
    "A".into()
}
const fn default_cert_port() -> i64 {
    443
}
const fn default_warning_days() -> i64 {
    30
}
const fn default_critical_days() -> i64 {
    7
}
const fn default_cooldown() -> i64 {
    300
}

#[derive(Clone, Debug, FromRow, Serialize)]
pub struct MonitorCheck {
    pub id: String,
    pub monitor_id: String,
    pub status: String,
    pub latency_ms: Option<i64>,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
    pub checked_at: String,
    pub extra_json: Option<String>,
}
