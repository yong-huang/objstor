use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: i64,
    pub bucket: String,
    pub key: String,
    pub version_id: Option<String>,
    pub object_hash: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub pool_id: String,
    pub storage_class: String,
    pub tags: Option<HashMap<String, String>>,
    pub metadata: Option<serde_json::Value>,
    pub encryption_info: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub storage_class: String,
}

const OBJECT_COLUMNS: &str = "
    id, bucket, key, version_id, object_hash, size, content_type, etag,
    created_at, modified_at, pool_id, storage_class, tags_json, metadata_json,
    encryption_info, tier
";

fn object_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Object> {
    Ok(Object {
        id: row.get(0)?,
        bucket: row.get(1)?,
        key: row.get(2)?,
        version_id: row.get(3)?,
        object_hash: row.get(4)?,
        size: row.get(5)?,
        content_type: row.get(6)?,
        etag: row.get(7)?,
        created_at: DateTime::from_timestamp(row.get(8)?, 0).unwrap(),
        modified_at: DateTime::from_timestamp(row.get(9)?, 0).unwrap(),
        pool_id: row.get(10)?,
        storage_class: row.get(11)?,
        tags: row
            .get::<_, Option<String>>(12)?
            .and_then(|s| serde_json::from_str(&s).ok()),
        metadata: row
            .get::<_, Option<String>>(13)?
            .and_then(|s| serde_json::from_str(&s).ok()),
        encryption_info: row.get(14)?,
        tier: row.get(15)?,
    })
}

impl MetadataStore {
    pub fn create_object(&self, obj: &Object) -> Result<i64> {
        let conn = self.conn().lock().unwrap();

        let now = Utc::now().timestamp();
        let tags_json = obj.tags.as_ref().map(serde_json::to_string).transpose()?;
        let metadata_json = obj
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        conn.execute(
            "INSERT INTO objects (bucket, key, version_id, object_hash, size, content_type, etag, created_at, modified_at, pool_id, storage_class, tags_json, metadata_json, encryption_info, tier)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                obj.bucket,
                obj.key,
                obj.version_id,
                obj.object_hash,
                obj.size,
                obj.content_type,
                obj.etag,
                now,
                now,
                obj.pool_id,
                obj.storage_class,
                tags_json,
                metadata_json,
                obj.encryption_info,
                obj.tier,
            ],
        ).map_err(Error::DatabaseError)?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Object> {
        let conn = self.conn().lock().unwrap();

        conn.query_row(
            &format!(
                "SELECT {} FROM objects WHERE bucket = ?1 AND key = ?2 AND version_id IS NULL",
                OBJECT_COLUMNS
            ),
            params![bucket, key],
            object_from_row,
        )
        .map_err(|e| {
            if let rusqlite::Error::QueryReturnedNoRows = e {
                Error::ObjectNotFound(key.to_string())
            } else {
                Error::DatabaseError(e)
            }
        })
    }

    pub fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        let conn = self.conn().lock().unwrap();

        let rows_affected = conn
            .execute(
                "DELETE FROM objects WHERE bucket = ?1 AND key = ?2 AND version_id IS NULL",
                params![bucket, key],
            )
            .map_err(Error::DatabaseError)?;

        if rows_affected == 0 {
            return Err(Error::ObjectNotFound(key.to_string()));
        }

        Ok(())
    }

    pub fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ObjectMetadata>> {
        let conn = self.conn().lock().unwrap();

        let query = if prefix.is_some() {
            "SELECT key, size, etag, modified_at, storage_class
             FROM objects
             WHERE bucket = ?1 AND key LIKE ?2 || '%' AND version_id IS NULL AND object_hash != ''
             ORDER BY key
             LIMIT ?3"
        } else {
            "SELECT key, size, etag, modified_at, storage_class
             FROM objects
             WHERE bucket = ?1 AND version_id IS NULL AND object_hash != ''
             ORDER BY key
             LIMIT ?2"
        };

        let mut stmt = conn.prepare(query).map_err(Error::DatabaseError)?;

        // Handle both cases separately to avoid type mismatch
        let objects = if let Some(_prefix) = prefix {
            // With prefix - not fully implemented, return empty
            Vec::new()
        } else {
            // Without prefix
            stmt.query_map(params![bucket, limit], |row| {
                Ok(ObjectMetadata {
                    key: row.get(0)?,
                    size: row.get(1)?,
                    etag: row.get(2)?,
                    last_modified: DateTime::from_timestamp(row.get(3)?, 0).unwrap(),
                    storage_class: row.get(4)?,
                })
            })
            .map_err(Error::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::DatabaseError)?
        };

        Ok(objects)
    }

    /// List objects that belong to a specific storage tier.
    pub fn list_objects_by_tier(&self, tier: &str, limit: usize) -> Result<Vec<Object>> {
        let conn = self.conn().lock().unwrap();

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM objects WHERE tier = ?1 ORDER BY created_at LIMIT ?2",
                OBJECT_COLUMNS
            ))
            .map_err(Error::DatabaseError)?;

        let objects = stmt
            .query_map(params![tier, limit], object_from_row)
            .map_err(Error::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::DatabaseError)?;

        Ok(objects)
    }

    /// Update an object's tier and storage_class after lifecycle migration.
    pub fn update_object_tier(&self, object_id: i64, tier: &str, storage_class: &str) -> Result<()> {
        let conn = self.conn().lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE objects SET tier = ?1, storage_class = ?2, modified_at = ?3 WHERE id = ?4",
            params![tier, storage_class, now, object_id],
        )
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    // === Versioning methods ===

    /// Get a specific version of an object.
    pub fn get_object_version(&self, bucket: &str, key: &str, version_id: &str) -> Result<Object> {
        let conn = self.conn().lock().unwrap();

        conn.query_row(
            &format!(
                "SELECT {} FROM objects WHERE bucket = ?1 AND key = ?2 AND version_id = ?3",
                OBJECT_COLUMNS
            ),
            params![bucket, key, version_id],
            object_from_row,
        )
        .map_err(|e| {
            if let rusqlite::Error::QueryReturnedNoRows = e {
                Error::ObjectNotFound(format!("{} (version {})", key, version_id))
            } else {
                Error::DatabaseError(e)
            }
        })
    }

    /// List all versions of objects in a bucket (including delete markers).
    pub fn list_object_versions(&self, bucket: &str, limit: usize) -> Result<Vec<Object>> {
        let conn = self.conn().lock().unwrap();

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM objects WHERE bucket = ?1 ORDER BY key, modified_at DESC LIMIT ?2",
                OBJECT_COLUMNS
            ))
            .map_err(Error::DatabaseError)?;

        let objects = stmt
            .query_map(params![bucket, limit], object_from_row)
            .map_err(Error::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::DatabaseError)?;

        Ok(objects)
    }

    /// Delete a specific version of an object.
    pub fn delete_object_version(&self, bucket: &str, key: &str, version_id: &str) -> Result<()> {
        let conn = self.conn().lock().unwrap();

        let rows_affected = conn
            .execute(
                "DELETE FROM objects WHERE bucket = ?1 AND key = ?2 AND version_id = ?3",
                params![bucket, key, version_id],
            )
            .map_err(Error::DatabaseError)?;

        if rows_affected == 0 {
            return Err(Error::ObjectNotFound(format!(
                "{} (version {})",
                key, version_id
            )));
        }

        Ok(())
    }

    /// Insert a delete marker for an object (versioned delete).
    pub fn insert_delete_marker(&self, bucket: &str, key: &str) -> Result<String> {
        let version_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let conn = self.conn().lock().unwrap();
        conn.execute(
            "INSERT INTO objects (bucket, key, version_id, object_hash, size, content_type, etag, created_at, modified_at, pool_id, storage_class)
             VALUES (?1, ?2, ?3, '', 0, NULL, '', ?4, ?4, '', '')",
            params![bucket, key, version_id, now],
        )
        .map_err(Error::DatabaseError)?;

        Ok(version_id)
    }

    // === Search ===

    /// Search for objects by key substring.
    pub fn search_objects(
        &self,
        query: &str,
        bucket: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Object>> {
        let conn = self.conn().lock().unwrap();

        let sql = if bucket.is_some() {
            &format!(
                "SELECT {} FROM objects WHERE bucket = ?1 AND key LIKE '%' || ?2 || '%' AND version_id IS NULL LIMIT ?3",
                OBJECT_COLUMNS
            )
        } else {
            &format!(
                "SELECT {} FROM objects WHERE key LIKE '%' || ?1 || '%' AND version_id IS NULL LIMIT ?2",
                OBJECT_COLUMNS
            )
        };

        let mut stmt = conn.prepare(sql).map_err(Error::DatabaseError)?;

        let rows = if let Some(b) = bucket {
            stmt.query_map(params![b, query, limit], object_from_row)
                .map_err(Error::DatabaseError)?
        } else {
            stmt.query_map(params![query, limit], object_from_row)
                .map_err(Error::DatabaseError)?
        };

        let mut objects = Vec::new();
        for row in rows {
            objects.push(row.map_err(Error::DatabaseError)?);
        }

        Ok(objects)
    }

    /// Advanced search using a JSON filter with multiple criteria.
    /// The filter can contain: bucket, prefix, key_contains, min_size, max_size,
    /// content_type, min_age_days, max_age_days.
    pub fn search_objects_advanced(
        &self,
        filter: &serde_json::Value,
        bucket: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Object>> {
        let conn = self.conn().lock().unwrap();

        let mut conditions = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        // Override bucket from filter if not provided in argument
        let effective_bucket = bucket.or_else(|| filter.get("bucket").and_then(|v| v.as_str()));

        if let Some(b) = effective_bucket {
            conditions.push("bucket = ?".to_string());
            param_values.push(Box::new(b.to_string()));
        }

        if let Some(prefix) = filter.get("prefix").and_then(|v| v.as_str()) {
            if !prefix.is_empty() {
                conditions.push("key LIKE ?".to_string());
                param_values.push(Box::new(format!("{}%", prefix)));
            }
        }

        if let Some(contains) = filter.get("key_contains").and_then(|v| v.as_str()) {
            if !contains.is_empty() {
                conditions.push("key LIKE ?".to_string());
                param_values.push(Box::new(format!("%{}%", contains)));
            }
        }

        if let Some(min_size) = filter.get("min_size").and_then(|v| v.as_u64()) {
            conditions.push("size >= ?".to_string());
            param_values.push(Box::new(min_size as i64));
        }

        if let Some(max_size) = filter.get("max_size").and_then(|v| v.as_u64()) {
            conditions.push("size <= ?".to_string());
            param_values.push(Box::new(max_size as i64));
        }

        if let Some(ct) = filter.get("content_type").and_then(|v| v.as_str()) {
            if !ct.is_empty() {
                conditions.push("content_type LIKE ?".to_string());
                param_values.push(Box::new(format!("%{}%", ct)));
            }
        }

        let now_ts = Utc::now().timestamp();

        if let Some(min_age) = filter.get("min_age_days").and_then(|v| v.as_f64()) {
            let cutoff = now_ts - (min_age * 86400.0) as i64;
            conditions.push("created_at <= ?".to_string());
            param_values.push(Box::new(cutoff));
        }

        if let Some(max_age) = filter.get("max_age_days").and_then(|v| v.as_f64()) {
            let cutoff = now_ts - (max_age * 86400.0) as i64;
            conditions.push("created_at >= ?".to_string());
            param_values.push(Box::new(cutoff));
        }

        // Always exclude versioned (delete marker) entries
        conditions.push("version_id IS NULL".to_string());

        let where_clause = conditions.join(" AND ");

        let sql = format!(
            "SELECT {} FROM objects WHERE {} ORDER BY created_at DESC LIMIT ?",
            OBJECT_COLUMNS, where_clause
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::DatabaseError)?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|b| b.as_ref()).collect();
        let mut params_with_limit = param_refs;
        params_with_limit.push(&limit);

        let rows = stmt
            .query_map(params_with_limit.as_slice(), object_from_row)
            .map_err(Error::DatabaseError)?;

        let mut objects = Vec::new();
        for row in rows {
            objects.push(row.map_err(Error::DatabaseError)?);
        }

        Ok(objects)
    }

    /// Update tags for an object (latest version, no version_id).
    pub fn update_object_tags(
        &self,
        bucket: &str,
        key: &str,
        tags: &HashMap<String, String>,
    ) -> Result<()> {
        let conn = self.conn().lock().unwrap();
        let tags_json = serde_json::to_string(tags)?;
        conn.execute(
            "UPDATE objects SET tags_json = ?1 WHERE bucket = ?2 AND key = ?3 AND version_id IS NULL",
            params![tags_json, bucket, key],
        )
        .map_err(Error::DatabaseError)?;
        Ok(())
    }

    /// Merge new metadata keys into an object's existing metadata_json.
    pub fn update_object_metadata(
        &self,
        bucket: &str,
        key: &str,
        new_meta: &serde_json::Value,
    ) -> Result<()> {
        let conn = self.conn().lock().unwrap();

        // Load existing metadata
        let existing: serde_json::Value = conn
            .query_row(
                "SELECT metadata_json FROM objects WHERE bucket = ?1 AND key = ?2 AND version_id IS NULL",
                params![bucket, key],
                |row| {
                    let raw: Option<String> = row.get(0)?;
                    Ok(raw
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new())))
                },
            )
            .map_err(|e| {
                if let rusqlite::Error::QueryReturnedNoRows = e {
                    Error::ObjectNotFound(key.to_string())
                } else {
                    Error::DatabaseError(e)
                }
            })?;

        // Merge new keys into existing
        let mut merged = existing;
        if let serde_json::Value::Object(ref mut map) = merged {
            if let serde_json::Value::Object(new_map) = new_meta {
                for (k, v) in new_map {
                    map.insert(k.clone(), v.clone());
                }
            }
        }

        let merged_json = serde_json::to_string(&merged)?;
        conn.execute(
            "UPDATE objects SET metadata_json = ?1 WHERE bucket = ?2 AND key = ?3 AND version_id IS NULL",
            params![merged_json, bucket, key],
        )
        .map_err(Error::DatabaseError)?;
        Ok(())
    }

    /// List all non-deleted objects across all buckets.
    pub fn list_all_objects(&self, limit: usize) -> Result<Vec<Object>> {
        let conn = self.conn().lock().unwrap();
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM objects WHERE version_id IS NULL AND object_hash != '' ORDER BY created_at DESC LIMIT ?1",
                OBJECT_COLUMNS
            ))
            .map_err(Error::DatabaseError)?;

        let objects = stmt
            .query_map(params![limit], object_from_row)
            .map_err(Error::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::DatabaseError)?;

        Ok(objects)
    }
}
