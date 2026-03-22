use crate::error::{Error, Result};
use crate::scheduler::metrics::MetricsCollector;
use crate::storage::pool::{PoolStatus, StoragePool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub strategy: SchedulingStrategy,
    pub rebalance_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SchedulingStrategy {
    WeightedRoundRobin,
    LeastLoaded,
    ConsistentHash,
    Adaptive,
}

pub struct LoadBalancer {
    config: SchedulerConfig,
    round_robin_index: std::sync::atomic::AtomicUsize,
    metrics: MetricsCollector,
}

impl LoadBalancer {
    pub fn new(strategy: SchedulingStrategy) -> Self {
        Self {
            config: SchedulerConfig {
                strategy,
                rebalance_threshold: 0.2,
            },
            round_robin_index: std::sync::atomic::AtomicUsize::new(0),
            metrics: MetricsCollector::new(),
        }
    }

    pub fn select_pool<'a>(&self, pools: &'a [StoragePool], size: u64) -> Result<&'a StoragePool> {
        match self.config.strategy {
            SchedulingStrategy::LeastLoaded => self.least_loaded_strategy(pools, size),
            SchedulingStrategy::WeightedRoundRobin => {
                self.weighted_round_robin_strategy(pools, size)
            }
            SchedulingStrategy::Adaptive => self.adaptive_strategy(pools, size),
            SchedulingStrategy::ConsistentHash => self.consistent_hash_strategy(pools, size),
        }
    }

    /// Least loaded strategy: select healthy pool with lowest usage ratio
    fn least_loaded_strategy<'a>(&self, pools: &'a [StoragePool], size: u64) -> Result<&'a StoragePool> {
        pools
            .iter()
            .filter(|p| p.status == PoolStatus::Healthy)
            .filter(|p| p.capacity - p.used >= size)
            .min_by_key(|p| {
                let usage_ratio = p.used as f64 / p.capacity as f64;
                (usage_ratio * 1000.0) as u64
            })
            .ok_or_else(|| Error::NoAvailablePool)
    }

    /// Weighted round robin: weight by available space
    fn weighted_round_robin_strategy<'a>(&self, pools: &'a [StoragePool], size: u64) -> Result<&'a StoragePool> {
        let available_pools: Vec<_> = pools
            .iter()
            .filter(|p| p.status == PoolStatus::Healthy)
            .filter(|p| p.capacity - p.used >= size)
            .collect();

        if available_pools.is_empty() {
            return Err(Error::NoAvailablePool);
        }

        // Calculate total available space
        let total_available: u64 = available_pools.iter().map(|p| p.capacity - p.used).sum();

        // Simple weighted selection based on available space
        let mut cumulative = 0u64;
        let target = (self.round_robin_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            as u64)
            % total_available;

        for pool in &available_pools {
            cumulative += pool.capacity - pool.used;
            if target < cumulative {
                return Ok(pool);
            }
        }

        // Fallback to first pool
        Ok(available_pools[0])
    }

    /// Adaptive strategy: consider multiple factors
    fn adaptive_strategy<'a>(&self, pools: &'a [StoragePool], size: u64) -> Result<&'a StoragePool> {
        // Score each pool based on multiple factors:
        // 1. Available space (40%)
        // 2. Object count (20%)
        // 3. Recent I/O metrics (40%)

        let available_pools: Vec<_> = pools
            .iter()
            .filter(|p| p.status == PoolStatus::Healthy)
            .filter(|p| p.capacity - p.used >= size)
            .collect();

        if available_pools.is_empty() {
            return Err(Error::NoAvailablePool);
        }

        let best_pool = available_pools
            .iter()
            .max_by_key(|p| {
                let space_score = ((p.capacity - p.used) as f64 / p.capacity as f64) * 40.0;

                let obj_score = if p.config.max_objects > 0 {
                    (1.0 - (p.objects_count as f64 / p.config.max_objects as f64)) * 20.0
                } else {
                    20.0
                };

                let io_score = self.metrics.get_io_score(&p.id);

                (space_score + obj_score + io_score) as u64
            })
            .unwrap();

        Ok(best_pool)
    }

    /// Consistent hash strategy: use hash of object to select pool
    fn consistent_hash_strategy<'a>(&self, pools: &'a [StoragePool], _size: u64) -> Result<&'a StoragePool> {
        let available_pools: Vec<_> = pools
            .iter()
            .filter(|p| p.status == PoolStatus::Healthy)
            .collect();

        if available_pools.is_empty() {
            return Err(Error::NoAvailablePool);
        }

        // Use current timestamp for hash (in real impl, use object key)
        let hash = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let index = (hash as usize) % available_pools.len();

        Ok(available_pools[index])
    }
}
