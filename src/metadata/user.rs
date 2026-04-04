use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessKey {
    pub access_key_id: String,
    pub secret_key: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub status: String,
    pub policies: Vec<Policy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub effect: String,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

impl MetadataStore {
    pub fn create_access_key(
        &self,
        access_key_id: &str,
        secret_key_hash: &str,
        owner: &str,
    ) -> Result<AccessKey> {
        let conn = self.conn().lock().unwrap();
        let now = Utc::now().timestamp();

        conn.execute(
            "INSERT INTO access_keys (access_key_id, secret_key_hash, owner, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![access_key_id, secret_key_hash, owner, now],
        )
        .map_err(Error::DatabaseError)?;

        Ok(AccessKey {
            access_key_id: access_key_id.to_string(),
            secret_key: secret_key_hash.to_string(),
            owner: owner.to_string(),
            created_at: Utc::now(),
            status: "active".to_string(),
            policies: Vec::new(),
        })
    }

    pub fn get_access_key(&self, access_key_id: &str) -> Result<AccessKey> {
        let conn = self.conn().lock().unwrap();

        conn.query_row(
            "SELECT access_key_id, secret_key_hash, owner, created_at, status, policies_json
             FROM access_keys WHERE access_key_id = ?1 AND status = 'active'",
            params![access_key_id],
            |row| {
                Ok(AccessKey {
                    access_key_id: row.get(0)?,
                    secret_key: row.get(1)?,
                    owner: row.get(2)?,
                    created_at: DateTime::from_timestamp(row.get(3)?, 0).unwrap(),
                    status: row.get(4)?,
                    policies: row
                        .get::<_, Option<String>>(5)?
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default(),
                })
            },
        )
        .map_err(|e| {
            if let rusqlite::Error::QueryReturnedNoRows = e {
                Error::InvalidAccessKey
            } else {
                Error::DatabaseError(e)
            }
        })
    }

    pub fn list_access_keys(&self) -> Result<Vec<AccessKey>> {
        let conn = self.conn().lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT access_key_id, secret_key_hash, owner, created_at, status, policies_json
                 FROM access_keys",
            )
            .map_err(Error::DatabaseError)?;

        let keys = stmt
            .query_map([], |row| {
                Ok(AccessKey {
                    access_key_id: row.get(0)?,
                    secret_key: row.get(1)?,
                    owner: row.get(2)?,
                    created_at: DateTime::from_timestamp(row.get(3)?, 0).unwrap(),
                    status: row.get(4)?,
                    policies: row
                        .get::<_, Option<String>>(5)?
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default(),
                })
            })
            .map_err(Error::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::DatabaseError)?;

        Ok(keys)
    }

    pub fn delete_access_key(&self, access_key_id: &str) -> Result<()> {
        let conn = self.conn().lock().unwrap();

        conn.execute(
            "DELETE FROM access_keys WHERE access_key_id = ?1",
            params![access_key_id],
        )
        .map_err(Error::DatabaseError)?;

        Ok(())
    }

    pub fn update_access_key(&self, access_key_id: &str, new_secret_hash: &str) -> Result<()> {
        let conn = self.conn().lock().unwrap();

        let rows = conn
            .execute(
                "UPDATE access_keys SET secret_key_hash = ?1 WHERE access_key_id = ?2",
                params![new_secret_hash, access_key_id],
            )
            .map_err(Error::DatabaseError)?;

        if rows == 0 {
            return Err(Error::InvalidAccessKey);
        }
        Ok(())
    }
}
