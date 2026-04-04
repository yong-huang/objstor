use crate::storage::pool::PoolConfig;
use crate::storage::tier::StorageTier;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    #[serde(default)]
    pub pools: Vec<StoragePoolConfig>,
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePoolConfig {
    pub id: String,
    pub path: PathBuf,
    #[serde(default = "default_pool_capacity")]
    pub capacity: u64,
    #[serde(default = "default_max_objects")]
    pub max_objects: u64,
    #[serde(default)]
    pub quota_enabled: bool,
    #[serde(default = "default_pool_tier")]
    pub tier: String,
}

fn default_pool_capacity() -> u64 {
    100 * 1024 * 1024 * 1024 // 100GB
}

fn default_max_objects() -> u64 {
    1_000_000
}

fn default_pool_tier() -> String {
    "hot".to_string()
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
            pools: vec![
                StoragePoolConfig {
                    id: "pool-001".to_string(),
                    path: PathBuf::from("./data/pools/pool-001"),
                    capacity: 100 * 1024 * 1024 * 1024, // 100GB
                    max_objects: 1_000_000,
                    quota_enabled: false,
                    tier: "hot".to_string(),
                },
                StoragePoolConfig {
                    id: "pool-002".to_string(),
                    path: PathBuf::from("./data/pools/pool-002"),
                    capacity: 100 * 1024 * 1024 * 1024, // 100GB
                    max_objects: 1_000_000,
                    quota_enabled: false,
                    tier: "warm".to_string(),
                },
            ],
            scheduler: SchedulerConfig::default(),
        }
    }
}

impl StorageConfig {
    /// Load storage configuration from a file
    pub fn from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        let config: StorageConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save storage configuration to a file
    pub fn to_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Convert StoragePoolConfig to PoolConfig
    pub fn to_pool_configs(&self) -> Vec<PoolConfig> {
        self.pools
            .iter()
            .map(|pool_config| PoolConfig {
                id: pool_config.id.clone(),
                path: pool_config.path.clone(),
                capacity: pool_config.capacity,
                max_objects: pool_config.max_objects,
                quota_enabled: pool_config.quota_enabled,
                tier: StorageTier::from_str_lower(&pool_config.tier),
            })
            .collect()
    }

    /// Initialize storage directory structure
    pub fn init_directories(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create data directory
        fs::create_dir_all(&self.data_dir)?;

        // Create config directory
        let config_dir = self.data_dir.join("config");
        fs::create_dir_all(&config_dir)?;

        // Create pools directory
        let pools_dir = self.data_dir.join("pools");
        fs::create_dir_all(&pools_dir)?;

        // Create each pool directory
        for pool_config in &self.pools {
            fs::create_dir_all(&pool_config.path)?;

            // Create objects subdirectory
            let objects_dir = pool_config.path.join("objects");
            fs::create_dir_all(&objects_dir)?;

            // Create metadata subdirectory
            let metadata_dir = pool_config.path.join("metadata");
            fs::create_dir_all(&metadata_dir)?;
        }

        Ok(())
    }
}
