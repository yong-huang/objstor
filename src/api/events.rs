use serde_json::Value;
use tokio::sync::broadcast;

/// Global event bus for real-time notifications.
pub struct EventBus {
    tx: broadcast::Sender<Value>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.tx.subscribe()
    }

    pub fn emit(&self, event: Value) {
        let _ = self.tx.send(event);
    }

    pub fn emit_object_created(&self, bucket: &str, key: &str, size: u64, etag: &str) {
        self.emit(serde_json::json!({
            "type": "event",
            "event": "ObjectCreated",
            "data": {
                "bucket": bucket,
                "key": key,
                "size": size,
                "etag": etag,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        }));
    }

    pub fn emit_object_deleted(&self, bucket: &str, key: &str) {
        self.emit(serde_json::json!({
            "type": "event",
            "event": "ObjectDeleted",
            "data": {
                "bucket": bucket,
                "key": key,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        }));
    }

    pub fn emit_bucket_created(&self, bucket: &str) {
        self.emit(serde_json::json!({
            "type": "event",
            "event": "BucketCreated",
            "data": {
                "bucket": bucket,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        }));
    }

    pub fn emit_bucket_deleted(&self, bucket: &str) {
        self.emit(serde_json::json!({
            "type": "event",
            "event": "BucketDeleted",
            "data": {
                "bucket": bucket,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        }));
    }
}
