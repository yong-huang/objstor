use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::params;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub owner: String,
    pub region: String,
    pub versioning_enabled: bool,
    pub quota: Option<u64>,
    pub acl: Option<Acl>,
    pub preferred_pool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acl {
    pub owner: String,
    pub grants: Vec<Grant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub grantee: Grantee,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grantee {
    pub id: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketMetadata {
    pub name: String,
    pub object_count: u64,
    pub total_size: u64,
}

impl MetadataStore {
    pub fn create_bucket(
        &self,
        name: &str,
        owner: &str,
        region: Option<String>,
        preferred_pool: Option<String>,
    ) -> Result<Bucket> {
        let now = Utc::now().timestamp();
        let region_value = region.clone().unwrap_or_else(|| "us-east-1".to_string());

        let conn = self.conn().lock().unwrap();
        conn.execute(
            "INSERT INTO buckets (name, created_at, owner, region, preferred_pool)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, now, owner, region_value, preferred_pool],
        )
        .map_err(Error::DatabaseError)?;

        Ok(Bucket {
            name: name.to_string(),
            created_at: Utc::now(),
            owner: owner.to_string(),
            region: region.unwrap_or_else(|| "us-east-1".to_string()),
            versioning_enabled: false,
            quota: None,
            acl: None,
            preferred_pool,
        })
    }

    pub fn delete_bucket(&self, name: &str) -> Result<()> {
        let conn = self.conn().lock().unwrap();

        // Check if bucket is empty
        let count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM objects WHERE bucket = ?1",
                params![name],
                |row| row.get(0),
            )
            .map_err(Error::DatabaseError)?;

        if count > 0 {
            return Err(Error::BucketNotEmpty(name.to_string()));
        }

        conn.execute("DELETE FROM buckets WHERE name = ?1", params![name])
            .map_err(Error::DatabaseError)?;

        Ok(())
    }

    pub fn get_bucket(&self, name: &str) -> Result<Bucket> {
        let conn = self.conn().lock().unwrap();

        conn.query_row(
            "SELECT name, created_at, owner, region, versioning_enabled, quota, acl_json, preferred_pool
             FROM buckets WHERE name = ?1",
            params![name],
            |row| {
                Ok(Bucket {
                    name: row.get(0)?,
                    created_at: DateTime::from_timestamp(row.get(1)?, 0).unwrap(),
                    owner: row.get(2)?,
                    region: row.get(3)?,
                    versioning_enabled: row.get(4)?,
                    quota: row.get(5)?,
                    acl: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    preferred_pool: row.get(7)?,
                })
            },
        )
        .map_err(|e| {
            if let rusqlite::Error::QueryReturnedNoRows = e {
                Error::BucketNotFound(name.to_string())
            } else {
                Error::DatabaseError(e)
            }
        })
    }

    pub fn list_buckets(&self) -> Result<Vec<Bucket>> {
        let conn = self.conn().lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT name, created_at, owner, region, versioning_enabled, quota, acl_json, preferred_pool
                 FROM buckets",
            )
            .map_err(Error::DatabaseError)?;

        let buckets = stmt
            .query_map([], |row| {
                Ok(Bucket {
                    name: row.get(0)?,
                    created_at: DateTime::from_timestamp(row.get(1)?, 0).unwrap(),
                    owner: row.get(2)?,
                    region: row.get(3)?,
                    versioning_enabled: row.get(4)?,
                    quota: row.get(5)?,
                    acl: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    preferred_pool: row.get(7)?,
                })
            })
            .map_err(Error::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::DatabaseError)?;

        Ok(buckets)
    }

    pub fn bucket_exists(&self, name: &str) -> Result<bool> {
        let conn = self.conn().lock().unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM buckets WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
            .map_err(Error::DatabaseError)?;

        Ok(exists)
    }

    pub fn get_bucket_metadata(&self, name: &str) -> Result<BucketMetadata> {
        let conn = self.conn().lock().unwrap();

        conn.query_row(
            "SELECT
                (SELECT COUNT(*) FROM objects WHERE bucket = ?1) as object_count,
                (SELECT COALESCE(SUM(size), 0) FROM objects WHERE bucket = ?1) as total_size",
            params![name],
            |row| {
                Ok(BucketMetadata {
                    name: name.to_string(),
                    object_count: row.get(0)?,
                    total_size: row.get(1)?,
                })
            },
        )
        .map_err(|e| {
            if let rusqlite::Error::QueryReturnedNoRows = e {
                Error::BucketNotFound(name.to_string())
            } else {
                Error::DatabaseError(e)
            }
        })
    }
}
