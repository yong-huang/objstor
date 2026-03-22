use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConfig {
    pub enabled: bool,
    pub max_versions: Option<u32>,
    pub retention_days: Option<u32>,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_versions: None,
            retention_days: None,
        }
    }
}
