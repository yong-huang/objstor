use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_api_endpoint")]
    pub api_endpoint: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub auto_tag: bool,
}

fn default_api_endpoint() -> String {
    "http://127.0.0.1:7001".to_string()
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_timeout_secs() -> u64 {
    15
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_endpoint: default_api_endpoint(),
            api_key: String::new(),
            model: String::new(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            auto_tag: false,
        }
    }
}
