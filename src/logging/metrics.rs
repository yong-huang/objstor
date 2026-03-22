use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct RequestMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_bytes_sent: AtomicU64,
    total_bytes_received: AtomicU64,
}

impl RequestMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            total_bytes_sent: AtomicU64::new(0),
            total_bytes_received: AtomicU64::new(0),
        }
    }

    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_success(&self) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_bytes(&self, sent: u64, received: u64) {
        self.total_bytes_sent.fetch_add(sent, Ordering::Relaxed);
        self.total_bytes_received.fetch_add(received, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> RequestStats {
        RequestStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            total_bytes_sent: self.total_bytes_sent.load(Ordering::Relaxed),
            total_bytes_received: self.total_bytes_received.load(Ordering::Relaxed),
        }
    }
}

impl Default for RequestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}
