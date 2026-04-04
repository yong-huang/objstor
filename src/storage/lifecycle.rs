use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use crate::storage::pool_manager::PoolManager;
use crate::storage::tier::StorageTier;
use chrono::Utc;
use rusqlite::params;
use std::sync::Arc;
use tracing;

/// A rule that defines when objects should transition between tiers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LifecycleRule {
    /// Only apply to objects whose key starts with this prefix.
    pub prefix: String,
    /// The tier objects are currently in.
    pub source_tier: StorageTier,
    /// The tier to migrate objects to.
    pub destination_tier: StorageTier,
    /// Number of days after creation before transitioning.
    pub transition_days: u64,
}

/// Background engine that periodically checks objects and migrates them
/// between storage tiers based on lifecycle rules.
pub struct LifecycleEngine {
    rules: Vec<LifecycleRule>,
}

impl LifecycleEngine {
    pub fn new(rules: Vec<LifecycleRule>) -> Self {
        Self { rules }
    }

    /// Run one lifecycle cycle — evaluate all rules and migrate matching objects.
    pub async fn run_cycle(
        &self,
        db: &Arc<MetadataStore>,
        pool_manager: &Arc<PoolManager>,
    ) -> Result<LifecycleCycleResult> {
        let mut migrated = 0u64;
        let mut failed = 0u64;

        let now = Utc::now().timestamp();

        for rule in &self.rules {
            // Query objects matching this rule: correct tier, old enough, matching prefix
            let cutoff = now - (rule.transition_days as i64 * 86400);
            let source_class = rule.source_tier.to_s3_storage_class();

            let objects_to_migrate = self.query_candidates(db, &rule.prefix, source_class, cutoff)?;

            for obj in objects_to_migrate {
                match self
                    .migrate_object(db, pool_manager, &obj, &rule.destination_tier)
                    .await
                {
                    Ok(()) => {
                        migrated += 1;
                        tracing::info!(
                            "Lifecycle: migrated {}/{} from {} to {}",
                            obj.bucket,
                            obj.key,
                            rule.source_tier,
                            rule.destination_tier
                        );
                    }
                    Err(e) => {
                        failed += 1;
                        tracing::warn!(
                            "Lifecycle: failed to migrate {}/{}: {}",
                            obj.bucket,
                            obj.key,
                            e
                        );
                    }
                }
            }
        }

        tracing::info!(
            "Lifecycle cycle complete: {} migrated, {} failed",
            migrated,
            failed
        );

        Ok(LifecycleCycleResult { migrated, failed })
    }

    /// Query objects that match a lifecycle rule.
    fn query_candidates(
        &self,
        db: &MetadataStore,
        prefix: &str,
        source_class: &str,
        cutoff: i64,
    ) -> Result<Vec<CandidateObject>> {
        let conn = db.conn().lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, bucket, key, object_hash, size, pool_id
                 FROM objects
                 WHERE storage_class = ?1 AND created_at < ?2 AND key LIKE ?3 || '%'",
            )
            .map_err(Error::DatabaseError)?;

        let rows = stmt
            .query_map(params![source_class, cutoff, prefix], |row| {
                Ok(CandidateObject {
                    id: row.get(0)?,
                    bucket: row.get(1)?,
                    key: row.get(2)?,
                    object_hash: row.get(3)?,
                    size: row.get(4)?,
                    pool_id: row.get(5)?,
                })
            })
            .map_err(Error::DatabaseError)?;

        let mut objects = Vec::new();
        for row in rows {
            objects.push(row.map_err(Error::DatabaseError)?);
        }

        Ok(objects)
    }

    /// Migrate a single object to a new tier.
    async fn migrate_object(
        &self,
        db: &MetadataStore,
        pool_manager: &Arc<PoolManager>,
        obj: &CandidateObject,
        dest_tier: &StorageTier,
    ) -> Result<()> {
        // Read data from current pool
        let source_pool = pool_manager.get_pool(&obj.pool_id).await?;
        let data = source_pool.read_object(&obj.object_hash).await?;

        // Select destination pool for the target tier
        let dest_class = dest_tier.to_s3_storage_class();
        let mut dest_pool = pool_manager
            .select_pool_for_tier(dest_tier, obj.size)
            .await?;

        // Write data to destination pool
        let dest_location = dest_pool.write_object(&data, dest_tier).await?;

        // Update metadata: pool_id and storage_class
        let conn = db.conn().lock().unwrap();
        conn.execute(
            "UPDATE objects SET pool_id = ?1, storage_class = ?2, modified_at = ?3 WHERE id = ?4",
            params![dest_location.pool_id, dest_class, Utc::now().timestamp(), obj.id],
        )
        .map_err(Error::DatabaseError)?;

        // Note: We don't delete from source pool here because dedup ref_counts
        // may have other references. The source data will be cleaned up when
        // ref_count reaches zero via normal delete flow.

        Ok(())
    }
}

#[derive(Debug)]
struct CandidateObject {
    id: i64,
    bucket: String,
    key: String,
    object_hash: String,
    size: u64,
    pool_id: String,
}

/// Result of a single lifecycle cycle run.
#[derive(Debug)]
pub struct LifecycleCycleResult {
    pub migrated: u64,
    pub failed: u64,
}
