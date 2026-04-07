use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub mod ai;
pub mod server;
pub mod storage;

pub use ai::AiConfig;
pub use server::ServerConfig;
pub use storage::{StorageConfig, StoragePoolConfig};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub ai: AiConfig,
}

impl Config {
    /// Load configuration from a JSON file
    pub fn from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a JSON file
    pub fn to_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load or create default configuration
    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = "data/config/objstor.json";

        if Path::new(config_path).exists() {
            Self::from_file(config_path)
        } else {
            // Create default config
            let config = Config::default();
            config.save_to_default_path()?;
            Ok(config)
        }
    }

    /// Save configuration to default path
    pub fn save_to_default_path(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config_path = "data/config/objstor.json";
        fs::create_dir_all("data/config")?;
        self.to_file(config_path)
    }
}
