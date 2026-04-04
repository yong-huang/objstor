use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use rusqlite::params;

/// Manages content-addressable deduplication via reference counting.
///
/// When the same data (identified by SHA-256 hash) is stored multiple times,
/// only one physical copy is kept. The `ref_counts` table tracks how many
/// logical objects reference each physical copy.
pub struct DedupManager;

impl DedupManager {
    /// Increment the reference count for a physical object.
    ///
    /// Returns `true` if the object already existed (dedup hit), meaning the
    /// caller should skip writing the physical data. Returns `false` if this
    /// is the first reference (dedup miss), meaning the caller should write.
    ///
    /// Uses `INSERT ... ON CONFLICT UPDATE` for atomicity.
    pub fn increment_ref_count(
        db: &MetadataStore,
        pool_id: &str,
        object_hash: &str,
        size: u64,
    ) -> Result<bool> {
        let conn = db.conn().lock().unwrap();

        // Try to increment existing row
        let updated = conn
            .execute(
                "UPDATE ref_counts SET ref_count = ref_count + 1
                 WHERE pool_id = ?1 AND object_hash = ?2",
                params![pool_id, object_hash],
            )
            .map_err(Error::DatabaseError)?;

        if updated > 0 {
            return Ok(true); // already exists — dedup hit
        }

        // Insert new row
        conn.execute(
            "INSERT INTO ref_counts (pool_id, object_hash, ref_count, size)
             VALUES (?1, ?2, 1, ?3)",
            params![pool_id, object_hash, size],
        )
        .map_err(Error::DatabaseError)?;

        Ok(false) // new object — dedup miss
    }

    /// Decrement the reference count for a physical object.
    ///
    /// Returns `true` if the ref_count reached zero, meaning the caller
    /// should physically delete the data. Returns `false` if references remain.
    pub fn decrement_ref_count(
        db: &MetadataStore,
        pool_id: &str,
        object_hash: &str,
    ) -> Result<bool> {
        let conn = db.conn().lock().unwrap();

        conn.execute(
            "UPDATE ref_counts SET ref_count = ref_count - 1
             WHERE pool_id = ?1 AND object_hash = ?2 AND ref_count > 0",
            params![pool_id, object_hash],
        )
        .map_err(Error::DatabaseError)?;

        // Read back the count
        let count: i64 = conn
            .query_row(
                "SELECT ref_count FROM ref_counts WHERE pool_id = ?1 AND object_hash = ?2",
                params![pool_id, object_hash],
                |row| row.get(0),
            )
            .map_err(Error::DatabaseError)?;

        if count <= 0 {
            // Clean up the ref_counts row
            conn.execute(
                "DELETE FROM ref_counts WHERE pool_id = ?1 AND object_hash = ?2",
                params![pool_id, object_hash],
            )
            .map_err(Error::DatabaseError)?;
            Ok(true) // should delete physical data
        } else {
            Ok(false) // references remain
        }
    }

    /// Get the current reference count for a physical object.
    pub fn get_ref_count(
        db: &MetadataStore,
        pool_id: &str,
        object_hash: &str,
    ) -> Result<i64> {
        let conn = db.conn().lock().unwrap();

        conn.query_row(
            "SELECT ref_count FROM ref_counts WHERE pool_id = ?1 AND object_hash = ?2",
            params![pool_id, object_hash],
            |row| row.get(0),
        )
        .map_err(Error::DatabaseError)
    }
}
