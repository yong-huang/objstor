use crate::error::Result;
use crate::storage::pool::StoragePool;

pub trait PlacementStrategy {
    fn select_pools(&self, pools: &[StoragePool], replication_factor: usize) -> Result<Vec<StoragePool>>;
}

pub struct SimpleReplication;

impl PlacementStrategy for SimpleReplication {
    fn select_pools(&self, pools: &[StoragePool], replication_factor: usize) -> Result<Vec<StoragePool>> {
        if pools.len() < replication_factor {
            return Err(crate::error::Error::ConfigurationError(
                "Not enough pools for replication".to_string(),
            ));
        }

        // Select first N healthy pools
        let selected: Vec<_> = pools
            .iter()
            .filter(|p| p.status == crate::storage::pool::PoolStatus::Healthy)
            .take(replication_factor)
            .cloned()
            .collect();

        if selected.len() < replication_factor {
            return Err(crate::error::Error::StorageUnavailable(
                "Not enough healthy pools".to_string(),
            ));
        }

        Ok(selected)
    }
}
