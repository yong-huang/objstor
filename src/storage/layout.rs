use std::path::PathBuf;

pub struct StorageLayout {
    pub data_dir: PathBuf,
}

impl StorageLayout {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn pools_dir(&self) -> PathBuf {
        self.data_dir.join("pools")
    }

    pub fn pool_dir(&self, pool_id: &str) -> PathBuf {
        self.pools_dir().join(pool_id)
    }

    pub fn objects_dir(&self, pool_id: &str) -> PathBuf {
        self.pool_dir(pool_id).join("objects")
    }

    pub fn object_dir(&self, pool_id: &str, hash: &str) -> PathBuf {
        let prefix = &hash[0..2];
        self.objects_dir(pool_id).join(prefix).join(hash)
    }

    pub fn config_dir(&self) -> PathBuf {
        self.data_dir.join("config")
    }

    pub fn init(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(self.pools_dir())?;
        std::fs::create_dir_all(self.config_dir())?;
        Ok(())
    }
}
