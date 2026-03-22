use crate::error::{Error, Result};
use crate::scheduler::{LoadBalancer, SchedulingStrategy};
use crate::storage::pool::{PoolConfig, StoragePool};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct PoolManager {
    pools: Arc<RwLock<Vec<StoragePool>>>,
    load_balancer: Arc<RwLock<LoadBalancer>>,
}

impl PoolManager {
    pub async fn new(pool_configs: Vec<PoolConfig>, strategy: SchedulingStrategy) -> Result<Self> {
        let mut pools = Vec::new();
        for config in pool_configs {
            let pool = StoragePool::new(config)?;
            pools.push(pool);
        }

        let load_balancer = LoadBalancer::new(strategy.clone());

        Ok(Self {
            pools: Arc::new(RwLock::new(pools)),
            load_balancer: Arc::new(RwLock::new(load_balancer)),
        })
    }

    pub async fn select_pool_for_object(&self, size: u64) -> Result<StoragePool> {
        let pools = self.pools.read().await;
        let pool = self.load_balancer.read().await.select_pool(&pools, size)?;
        Ok(pool.clone())
    }

    /// Select pool for a bucket, optionally using preferred pool
    pub async fn select_pool_for_bucket(
        &self,
        preferred_pool: Option<&str>,
        size: u64,
    ) -> Result<StoragePool> {
        if let Some(pool_id) = preferred_pool {
            // Use the preferred pool if specified
            self.get_pool(pool_id).await
        } else {
            // Otherwise use load balancer to select best pool
            self.select_pool_for_object(size).await
        }
    }

    pub async fn get_pool(&self, pool_id: &str) -> Result<StoragePool> {
        let pools = self.pools.read().await;
        pools
            .iter()
            .find(|p| p.id == pool_id)
            .cloned()
            .ok_or_else(|| Error::StorageNotFound(pool_id.to_string()))
    }

    pub async fn get_all_pools(&self) -> Vec<StoragePool> {
        self.pools.read().await.clone()
    }

    pub async fn get_total_usage(&self) -> (u64, u64) {
        let pools = self.pools.read().await;
        let total_used: u64 = pools.iter().map(|p| p.used).sum();
        let total_capacity: u64 = pools.iter().map(|p| p.capacity).sum();
        (total_used, total_capacity)
    }

    pub async fn add_pool(&self, config: PoolConfig) -> Result<()> {
        let pool = StoragePool::new(config)?;
        self.pools.write().await.push(pool);
        Ok(())
    }

    pub async fn remove_pool(&self, pool_id: &str) -> Result<()> {
        let mut pools = self.pools.write().await;
        let pos = pools
            .iter()
            .position(|p| p.id == pool_id)
            .ok_or_else(|| Error::StorageNotFound(pool_id.to_string()))?;
        pools.remove(pos);
        Ok(())
    }
}
