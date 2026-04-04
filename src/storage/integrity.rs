use crate::api::events::EventBus;
use crate::error::Result;
use crate::storage::pool_manager::PoolManager;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing;

/// Result of a single integrity check run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IntegrityResult {
    pub checked: u64,
    pub mismatches: u64,
    pub errors: u64,
    pub timestamp: String,
    pub details: Vec<IntegrityMismatch>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IntegrityMismatch {
    pub pool_id: String,
    pub object_hash: String,
    pub expected: String,
    pub actual: String,
}

/// Background integrity checker that verifies object hashes.
pub struct IntegrityChecker;

impl IntegrityChecker {
    /// Run a full integrity check on all pools.
    ///
    /// Reads every physical object, recomputes SHA-256, compares with the stored hash.
    pub async fn run_check(
        pool_manager: &Arc<PoolManager>,
        event_bus: &Arc<EventBus>,
    ) -> Result<IntegrityResult> {
        let pools = pool_manager.get_all_pools().await;
        let mut checked = 0u64;
        let mut mismatches = 0u64;
        let mut errors = 0u64;
        let mut details = Vec::new();

        for pool in &pools {
            let objects_dir = pool.path.join("objects");
            if !objects_dir.exists() {
                continue;
            }

            // Walk prefix directories (2-hex)
            if let Ok(entries) = std::fs::read_dir(&objects_dir) {
                for prefix_entry in entries.flatten() {
                    if !prefix_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }

                    if let Ok(hash_entries) = std::fs::read_dir(prefix_entry.path()) {
                        for hash_entry in hash_entries.flatten() {
                            if !hash_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                                continue;
                            }

                            let object_hash = hash_entry
                                .file_name()
                                .to_string_lossy()
                                .to_string();
                            let data_path = hash_entry.path().join("data");

                            checked += 1;

                            match std::fs::read(&data_path) {
                                Ok(data) => {
                                    let mut hasher = Sha256::new();
                                    hasher.update(&data);
                                    let actual = hex::encode(hasher.finalize());

                                    if actual != object_hash {
                                        mismatches += 1;
                                        let hash_copy = object_hash.clone();
                                        details.push(IntegrityMismatch {
                                            pool_id: pool.id.clone(),
                                            object_hash: hash_copy.clone(),
                                            expected: hash_copy,
                                            actual,
                                        });
                                        tracing::warn!(
                                            "Integrity: mismatch in pool {} hash {}",
                                            pool.id,
                                            object_hash
                                        );
                                    }
                                }
                                Err(_) => {
                                    errors += 1;
                                    tracing::warn!(
                                        "Integrity: failed to read pool {} hash {}",
                                        pool.id,
                                        object_hash
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        let result = IntegrityResult {
            checked,
            mismatches,
            errors,
            timestamp: chrono::Utc::now().to_rfc3339(),
            details,
        };

        tracing::info!(
            "Integrity check complete: {} checked, {} mismatches, {} errors",
            result.checked,
            result.mismatches,
            result.errors
        );

        // Emit event
        event_bus.emit(serde_json::json!({
            "type": "event",
            "event": "IntegrityCheck",
            "data": {
                "checked": result.checked,
                "mismatches": result.mismatches,
                "errors": result.errors,
                "timestamp": result.timestamp,
            }
        }));

        Ok(result)
    }
}
