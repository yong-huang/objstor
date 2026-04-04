use crate::error::{Error, Result};
use crate::metadata::db::MetadataStore;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub status_code: i32,
    pub bucket: Option<String>,
    pub key: Option<String>,
    pub access_key: Option<String>,
    pub source_ip: Option<String>,
    pub user_agent: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: i64,
}

impl MetadataStore {
    /// Insert an audit log entry.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_audit_log(
        &self,
        method: &str,
        path: &str,
        status_code: u16,
        bucket: Option<&str>,
        key: Option<&str>,
        access_key: Option<&str>,
        source_ip: Option<&str>,
        user_agent: Option<&str>,
        error_message: Option<&str>,
        duration_ms: i64,
    ) -> Result<()> {
        let conn = self.conn().lock().unwrap();
        conn.execute(
            "INSERT INTO audit_logs (timestamp, method, path, status_code, bucket, key, access_key, source_ip, user_agent, error_message, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                chrono::Utc::now().to_rfc3339(),
                method,
                path,
                status_code as i32,
                bucket,
                key,
                access_key,
                source_ip,
                user_agent,
                error_message,
                duration_ms,
            ],
        )
        .map_err(Error::DatabaseError)?;
        Ok(())
    }

    /// Query audit logs, optionally filtered by bucket.
    pub fn query_audit_logs(&self, limit: usize, bucket: Option<&str>) -> Result<Vec<AuditLog>> {
        let conn = self.conn().lock().unwrap();

        let logs = if let Some(b) = bucket {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, method, path, status_code, bucket, key, access_key, source_ip, user_agent, error_message, duration_ms
                     FROM audit_logs WHERE bucket = ?1 ORDER BY id DESC LIMIT ?2",
                )
                .map_err(Error::DatabaseError)?;

            let rows: Vec<AuditLog> = stmt
                .query_map(params![b, limit], |row| {
                    Ok(AuditLog {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        method: row.get(2)?,
                        path: row.get(3)?,
                        status_code: row.get(4)?,
                        bucket: row.get(5)?,
                        key: row.get(6)?,
                        access_key: row.get(7)?,
                        source_ip: row.get(8)?,
                        user_agent: row.get(9)?,
                        error_message: row.get(10)?,
                        duration_ms: row.get(11)?,
                    })
                })
                .map_err(Error::DatabaseError)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Error::DatabaseError)?;
            rows
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, method, path, status_code, bucket, key, access_key, source_ip, user_agent, error_message, duration_ms
                     FROM audit_logs ORDER BY id DESC LIMIT ?1",
                )
                .map_err(Error::DatabaseError)?;

            let rows: Vec<AuditLog> = stmt
                .query_map(params![limit], |row| {
                    Ok(AuditLog {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        method: row.get(2)?,
                        path: row.get(3)?,
                        status_code: row.get(4)?,
                        bucket: row.get(5)?,
                        key: row.get(6)?,
                        access_key: row.get(7)?,
                        source_ip: row.get(8)?,
                        user_agent: row.get(9)?,
                        error_message: row.get(10)?,
                        duration_ms: row.get(11)?,
                    })
                })
                .map_err(Error::DatabaseError)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(Error::DatabaseError)?;
            rows
        };

        Ok(logs)
    }
}
