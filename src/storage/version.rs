use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub max_versions: Option<u32>,
    #[serde(default)]
    pub retention_days: Option<u32>,
}
