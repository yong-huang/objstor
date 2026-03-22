use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub read_ops: u64,
    pub write_ops: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub avg_latency_ms: f64,
    pub last_updated: Instant,
}

impl Default for PoolMetrics {
    fn default() -> Self {
        Self {
            read_ops: 0,
            write_ops: 0,
            bytes_read: 0,
            bytes_written: 0,
            avg_latency_ms: 0.0,
            last_updated: Instant::now(),
        }
    }
}

#[derive(Clone)]
pub struct MetricsCollector {
    pool_metrics: Arc<RwLock<HashMap<String, PoolMetrics>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            pool_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn record_read(&self, pool_id: &str, bytes: u64, latency_ms: f64) {
        let mut metrics = self.pool_metrics.write().unwrap();
        let entry = metrics.entry(pool_id.to_string()).or_default();
        entry.read_ops += 1;
        entry.bytes_read += bytes;
        entry.avg_latency_ms = (entry.avg_latency_ms * (entry.read_ops - 1) as f64 + latency_ms)
            / entry.read_ops as f64;
        entry.last_updated = Instant::now();
    }

    pub fn record_write(&self, pool_id: &str, bytes: u64, latency_ms: f64) {
        let mut metrics = self.pool_metrics.write().unwrap();
        let entry = metrics.entry(pool_id.to_string()).or_default();
        entry.write_ops += 1;
        entry.bytes_written += bytes;
        entry.avg_latency_ms = (entry.avg_latency_ms * (entry.write_ops - 1) as f64 + latency_ms)
            / entry.write_ops as f64;
        entry.last_updated = Instant::now();
    }

    pub fn get_metrics(&self, pool_id: &str) -> Option<PoolMetrics> {
        self.pool_metrics.read().unwrap().get(pool_id).cloned()
    }

    pub fn get_io_score(&self, pool_id: &str) -> f64 {
        if let Some(metrics) = self.get_metrics(pool_id) {
            // Score based on latency (lower is better)
            // Normalize: assume 100ms is bad, 1ms is good
            let latency_score = (1.0 - (metrics.avg_latency_ms / 100.0).min(1.0)) * 40.0;
            latency_score.max(0.0)
        } else {
            20.0 // Neutral score for pools with no metrics
        }
    }

    pub fn reset_metrics(&self, pool_id: &str) {
        let mut metrics = self.pool_metrics.write().unwrap();
        if let Some(entry) = metrics.get_mut(pool_id) {
            *entry = PoolMetrics::default();
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
