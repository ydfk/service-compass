pub mod cert;
pub mod dns;
pub mod docker;
pub mod http;
pub mod scheduler;
pub mod status;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CheckResult {
    pub status: String,
    pub latency_ms: Option<i64>,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
    pub extra_json: Option<String>,
}

impl CheckResult {
    pub fn down(message: impl Into<String>) -> Self {
        Self {
            status: "down".into(),
            latency_ms: None,
            status_code: None,
            error_message: Some(message.into()),
            extra_json: None,
        }
    }
}
