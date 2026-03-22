use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLog {
    pub timestamp: DateTime<Utc>,
    pub bucket: Option<String>,
    pub key: Option<String>,
    pub operation: String,
    pub http_method: String,
    pub http_status: u16,
    pub request_id: String,
    pub error_code: Option<String>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub duration_ms: u64,
    pub user_agent: Option<String>,
    pub access_key: Option<String>,
}

impl AccessLog {
    pub fn new(
        operation: String,
        http_method: String,
        request_id: String,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            bucket: None,
            key: None,
            operation,
            http_method,
            http_status: 200,
            request_id,
            error_code: None,
            bytes_sent: 0,
            bytes_received: 0,
            duration_ms: 0,
            user_agent: None,
            access_key: None,
        }
    }

    pub fn log(&self) {
        tracing::info!(
            timestamp = %self.timestamp,
            bucket = self.bucket.as_deref().unwrap_or("-"),
            key = self.key.as_deref().unwrap_or("-"),
            operation = %self.operation,
            method = %self.http_method,
            status = self.http_status,
            request_id = %self.request_id,
            error = self.error_code.as_deref().unwrap_or("-"),
            bytes_sent = self.bytes_sent,
            bytes_received = self.bytes_received,
            duration_ms = self.duration_ms,
            user_agent = self.user_agent.as_deref().unwrap_or("-"),
            access_key = self.access_key.as_deref().unwrap_or("-"),
            target = "access_log",
        );
    }
}
