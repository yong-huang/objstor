pub mod server;
pub mod storage;

pub use server::ServerConfig;
pub use storage::StorageConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
}
