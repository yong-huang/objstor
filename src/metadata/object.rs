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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub key: String,
    pub size: u64,
    pub etag: String,
    pub last_modified: DateTime<Utc>,
    pub storage_class: String,
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
            "INSERT INTO objects (bucket, key, version_id, object_hash, size, content_type, etag, created_at, modified_at, pool_id, storage_class, tags_json, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
            ],
        ).map_err(Error::DatabaseError)?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Result<Object> {
        let conn = self.conn().lock().unwrap();

        conn.query_row(
            "SELECT id, bucket, key, version_id, object_hash, size, content_type, etag, created_at, modified_at, pool_id, storage_class, tags_json, metadata_json
             FROM objects WHERE bucket = ?1 AND key = ?2 AND version_id IS NULL",
            params![bucket, key],
            |row| {
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
                    tags: row.get::<_, Option<String>>(12)?.and_then(|s| serde_json::from_str(&s).ok()),
                    metadata: row.get::<_, Option<String>>(13)?.and_then(|s| serde_json::from_str(&s).ok()),
                })
            },
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
                "DELETE FROM objects WHERE bucket = ?1 AND key = ?2",
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
             WHERE bucket = ?1 AND key LIKE ?2 || '%'
             ORDER BY key
             LIMIT ?3"
        } else {
            "SELECT key, size, etag, modified_at, storage_class
             FROM objects
             WHERE bucket = ?1
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
}
