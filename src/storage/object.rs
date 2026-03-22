use crate::error::Result;
use crate::storage::PoolManager;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Object {
    pub bucket: String,
    pub key: String,
    pub version_id: Option<String>,
    pub size: u64,
    pub content_type: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub storage_class: String,
}

pub struct ObjectWriter {
    pool_manager: PoolManager,
    bucket: String,
    key: String,
    data: Vec<u8>,
}

impl ObjectWriter {
    pub fn new(pool_manager: PoolManager, bucket: String, key: String) -> Self {
        Self {
            pool_manager,
            bucket,
            key,
            data: Vec::new(),
        }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        self.data.extend_from_slice(data);
        Ok(())
    }

    pub async fn finish(self) -> Result<ObjectLocation> {
        let size = self.data.len() as u64;
        let mut pool = self.pool_manager.select_pool_for_object(size).await?;
        let location = pool.write_object(&self.data).await?;

        Ok(ObjectLocation {
            pool_id: location.pool_id,
            object_hash: location.object_hash,
            path: location.path,
            size: location.size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ObjectLocation {
    pub pool_id: String,
    pub object_hash: String,
    pub path: std::path::PathBuf,
    pub size: u64,
}
