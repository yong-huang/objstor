use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

/// A single token bucket for rate limiting.
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: usize, refill_rate: f64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}

/// Per-bucket rate limiter using token-bucket algorithm.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    default_rps: f64,
    burst: usize,
}

impl RateLimiter {
    pub fn new(default_rps: f64, burst: usize) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            default_rps,
            burst,
        }
    }

    /// Check if a request for the given bucket is allowed.
    /// Returns true if allowed, false if rate limit exceeded.
    pub fn try_consume(&self, bucket: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        buckets
            .entry(bucket.to_string())
            .or_insert_with(|| TokenBucket::new(self.burst, self.default_rps))
            .try_consume()
    }
}
