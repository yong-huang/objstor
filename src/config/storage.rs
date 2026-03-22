use crate::storage::pool::PoolConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    #[serde(default)]
    pub pools: Vec<PoolConfig>,
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(default = "default_rebalance_threshold")]
    pub rebalance_threshold: f64,
}

fn default_strategy() -> String {
    "least_loaded".to_string()
}

fn default_rebalance_threshold() -> f64 {
    0.2
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            strategy: default_strategy(),
            rebalance_threshold: default_rebalance_threshold(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            pools: Vec::new(),
            scheduler: SchedulerConfig::default(),
        }
    }
}
