use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub s3_port: u16,
    #[serde(default)]
    pub enable_tls: bool,
    #[serde(default)]
    pub tls_cert: String,
    #[serde(default)]
    pub tls_key: String,
    #[serde(default = "default_max_request_size")]
    pub max_request_size: usize,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,
}

fn default_max_request_size() -> usize {
    5 * 1024 * 1024 * 1024 // 5GB
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_dir() -> PathBuf {
    PathBuf::from("./logs")
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            s3_port: 8080,
            enable_tls: false,
            tls_cert: String::new(),
            tls_key: String::new(),
            max_request_size: default_max_request_size(),
            log_level: default_log_level(),
            log_dir: default_log_dir(),
        }
    }
}
