use crate::error::{Error, Result};
use crate::storage::tier::StorageTier;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePool {
    pub id: String,
    pub path: PathBuf,
    pub capacity: u64,
    pub used: u64,
    pub objects_count: u64,
    pub status: PoolStatus,
    pub config: PoolConfig,
    pub tier: StorageTier,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolStatus {
    Healthy,
    Degraded,
    Offline,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub id: String,
    pub path: PathBuf,
    #[serde(default = "default_capacity")]
    pub capacity: u64,
    #[serde(default = "default_max_objects")]
    pub max_objects: u64,
    #[serde(default)]
    pub quota_enabled: bool,
    #[serde(default = "default_tier")]
    pub tier: StorageTier,
}

fn default_capacity() -> u64 {
    100 * 1024 * 1024 * 1024 // 100GB
}

fn default_max_objects() -> u64 {
    1_000_000
}

fn default_tier() -> StorageTier {
    StorageTier::Hot
}

#[derive(Debug, Clone)]
pub struct ObjectLocation {
    pub pool_id: String,
    pub object_hash: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct PoolMetadata {
    id: String,
    capacity: u64,
    used: u64,
    objects_count: u64,
    status: String,
}

impl StoragePool {
    pub fn new(config: PoolConfig) -> Result<Self> {
        // Create pool directory structure
        let objects_dir = config.path.join("objects");
        let metadata_dir = config.path.join("metadata");

        fs::create_dir_all(&objects_dir).map_err(Error::IoError)?;
        fs::create_dir_all(&metadata_dir).map_err(Error::IoError)?;

        // Load or create metadata
        let metadata_path = metadata_dir.join("pool.json");
        let (used, objects_count, status) = if metadata_path.exists() {
            let metadata: PoolMetadata =
                serde_json::from_slice(&fs::read(&metadata_path).map_err(Error::IoError)?)
                    .map_err(Error::SerializationError)?;
            let status = match metadata.status.as_str() {
                "Healthy" => PoolStatus::Healthy,
                "Degraded" => PoolStatus::Degraded,
                "Offline" => PoolStatus::Offline,
                "Full" => PoolStatus::Full,
                _ => PoolStatus::Healthy,
            };
            (metadata.used, metadata.objects_count, status)
        } else {
            (0, 0, PoolStatus::Healthy)
        };

        Ok(Self {
            id: config.id.clone(),
            path: config.path.clone(),
            capacity: config.capacity,
            used,
            objects_count,
            status,
            tier: config.tier,
            config,
        })
    }

    pub fn allocate(&self, size: u64) -> Result<ObjectLocation> {
        // Check capacity
        if self.used + size > self.capacity {
            return Err(Error::StorageFull(self.id.clone()));
        }

        // Check object count limit
        if self.objects_count >= self.config.max_objects {
            return Err(Error::StorageFull(self.id.clone()));
        }

        // Check pool health
        if self.status != PoolStatus::Healthy {
            return Err(Error::StorageUnavailable(self.id.clone()));
        }

        // Generate placeholder for hash (will be set during write)
        Ok(ObjectLocation {
            pool_id: self.id.clone(),
            object_hash: String::new(),
            path: PathBuf::new(),
            size,
        })
    }

    /// Write an object to the pool. If the tier is not Hot, compress with zstd.
    /// The hash is computed on the physical bytes (possibly compressed).
    pub async fn write_object(&mut self, data: &[u8], tier: &StorageTier) -> Result<ObjectLocation> {
        // Optionally compress for non-Hot tiers
        let (physical_data, compressed) = if *tier != StorageTier::Hot && data.len() > 1024 {
            match zstd::encode_all(data, 3) {
                Ok(compressed) if compressed.len() < data.len() => (compressed, true),
                _ => (data.to_vec(), false),
            }
        } else {
            (data.to_vec(), false)
        };

        // Calculate hash on physical bytes
        let mut hasher = Sha256::new();
        hasher.update(&physical_data);
        let hash = hex::encode(hasher.finalize());

        // Create object path
        let prefix = &hash[0..2];
        let object_dir = self.path.join("objects").join(prefix).join(&hash);
        fs::create_dir_all(&object_dir).map_err(Error::IoError)?;

        let data_path = object_dir.join("data");
        let meta_path = object_dir.join("meta.json");

        // Write data
        let mut file = tokio::fs::File::create(&data_path)
            .await
            .map_err(Error::IoError)?;
        file.write_all(&physical_data).await.map_err(Error::IoError)?;
        file.flush().await.map_err(Error::IoError)?;

        // Write metadata
        let meta = PhysicalObjectMetadata {
            size: physical_data.len() as u64,
            created_at: chrono::Utc::now().to_rfc3339(),
            compression: if compressed {
                Some("zstd".to_string())
            } else {
                None
            },
        };
        let meta_json = serde_json::to_string_pretty(&meta).map_err(Error::SerializationError)?;
        tokio::fs::write(&meta_path, meta_json)
            .await
            .map_err(Error::IoError)?;

        // Update pool stats
        self.used += physical_data.len() as u64;
        self.objects_count += 1;
        self.save_metadata().await?;

        Ok(ObjectLocation {
            pool_id: self.id.clone(),
            object_hash: hash,
            path: data_path,
            size: physical_data.len() as u64,
        })
    }

    /// Legacy write_object without tier (always Hot, no compression).
    pub async fn write_object_legacy(&mut self, data: &[u8]) -> Result<ObjectLocation> {
        self.write_object(data, &StorageTier::Hot).await
    }

    /// Read an object from the pool. Automatically decompresses if stored compressed.
    pub async fn read_object(&self, hash: &str) -> Result<Vec<u8>> {
        let prefix = &hash[0..2];
        let object_dir = self
            .path
            .join("objects")
            .join(prefix)
            .join(hash);
        let data_path = object_dir.join("data");

        if !data_path.exists() {
            return Err(Error::ObjectNotFound(hash.to_string()));
        }

        let raw = tokio::fs::read(&data_path).await.map_err(Error::IoError)?;

        // Check if compressed
        let meta_path = object_dir.join("meta.json");
        if meta_path.exists() {
            let meta_content = tokio::fs::read_to_string(&meta_path)
                .await
                .map_err(Error::IoError)?;
            if let Ok(meta) = serde_json::from_str::<PhysicalObjectMetadata>(&meta_content) {
                if let Some(ref algo) = meta.compression {
                    if algo == "zstd" {
                        return zstd::decode_all(raw.as_slice()).map_err(|_| {
                            Error::InternalError("Decompression failed".to_string())
                        });
                    }
                }
            }
        }

        Ok(raw)
    }

    pub async fn delete_object(&mut self, hash: &str) -> Result<()> {
        let prefix = &hash[0..2];
        let object_dir = self.path.join("objects").join(prefix).join(hash);

        if !object_dir.exists() {
            return Err(Error::ObjectNotFound(hash.to_string()));
        }

        // Read metadata to get size
        let meta_path = object_dir.join("meta.json");
        let size = if meta_path.exists() {
            let meta_content = tokio::fs::read_to_string(&meta_path)
                .await
                .map_err(Error::IoError)?;
            let meta: PhysicalObjectMetadata =
                serde_json::from_str(&meta_content).map_err(Error::SerializationError)?;
            meta.size
        } else {
            0
        };

        // Remove object directory
        fs::remove_dir_all(&object_dir).map_err(Error::IoError)?;

        // Update pool stats
        self.used = self.used.saturating_sub(size);
        self.objects_count = self.objects_count.saturating_sub(1);
        self.save_metadata().await?;

        Ok(())
    }

    /// Register an already-existing physical object (dedup hit) by bumping pool stats.
    pub async fn register_existing_object(&mut self, size: u64) -> Result<()> {
        self.used += size;
        self.objects_count += 1;
        self.save_metadata().await
    }

    /// Unregister one logical reference (dedup-aware delete).
    pub async fn unregister_logical_object(&mut self, size: u64) -> Result<()> {
        self.used = self.used.saturating_sub(size);
        self.objects_count = self.objects_count.saturating_sub(1);
        self.save_metadata().await
    }

    pub fn get_usage(&self) -> (u64, u64) {
        (self.used, self.capacity)
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.used as f64 / self.capacity as f64
        }
    }

    pub async fn save_metadata(&self) -> Result<()> {
        let metadata_dir = self.path.join("metadata");
        let metadata_path = metadata_dir.join("pool.json");

        let metadata = PoolMetadata {
            id: self.id.clone(),
            capacity: self.capacity,
            used: self.used,
            objects_count: self.objects_count,
            status: format!("{:?}", self.status),
        };

        let metadata_json =
            serde_json::to_string_pretty(&metadata).map_err(Error::SerializationError)?;
        tokio::fs::write(&metadata_path, metadata_json)
            .await
            .map_err(Error::IoError)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PhysicalObjectMetadata {
    size: u64,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    compression: Option<String>,
}
