use crate::error::{Error, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
#[allow(dead_code)]
pub struct DbError(rusqlite::Error);

impl From<rusqlite::Error> for DbError {
    fn from(err: rusqlite::Error) -> Self {
        DbError(err)
    }
}

pub struct MetadataStore {
    conn: Arc<Mutex<Connection>>,
}

impl MetadataStore {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path).map_err(Error::DatabaseError)?;

        // Enable WAL mode for better concurrency (use query to handle returned results)
        conn.query_row("PRAGMA journal_mode=WAL", [], |_row| Ok(())).map_err(Error::DatabaseError)?;

        // Create tables
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        // Buckets table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS buckets (
                name TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                owner TEXT NOT NULL,
                region TEXT DEFAULT 'us-east-1',
                versioning_enabled INTEGER DEFAULT 0,
                quota INTEGER,
                acl_json TEXT,
                preferred_pool TEXT
            )",
            [],
        )
        .map_err(Error::DatabaseError)?;

        // Add preferred_pool column if table exists without it (for backward compatibility)
        let _ = conn.execute(
            "ALTER TABLE buckets ADD COLUMN preferred_pool TEXT",
            [],
        );

        // Objects table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS objects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bucket TEXT NOT NULL,
                key TEXT NOT NULL,
                version_id TEXT,
                object_hash TEXT NOT NULL,
                size INTEGER NOT NULL,
                content_type TEXT,
                etag TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                modified_at INTEGER NOT NULL,
                pool_id TEXT NOT NULL,
                storage_class TEXT DEFAULT 'STANDARD',
                tags_json TEXT,
                metadata_json TEXT,
                UNIQUE(bucket, key, version_id)
            )",
            [],
        )
        .map_err(Error::DatabaseError)?;

        // Multipart uploads table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS multipart_uploads (
                upload_id TEXT PRIMARY KEY,
                bucket TEXT NOT NULL,
                key TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                owner TEXT NOT NULL,
                content_type TEXT,
                metadata_json TEXT
            )",
            [],
        )
        .map_err(Error::DatabaseError)?;

        // Upload parts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS upload_parts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                upload_id TEXT NOT NULL,
                part_number INTEGER NOT NULL,
                object_hash TEXT NOT NULL,
                size INTEGER NOT NULL,
                etag TEXT NOT NULL,
                FOREIGN KEY (upload_id) REFERENCES multipart_uploads(upload_id)
            )",
            [],
        )
        .map_err(Error::DatabaseError)?;

        // Access keys table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS access_keys (
                access_key_id TEXT PRIMARY KEY,
                secret_key_hash TEXT NOT NULL,
                owner TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                status TEXT DEFAULT 'active',
                policies_json TEXT
            )",
            [],
        )
        .map_err(Error::DatabaseError)?;

        // Create indexes
        conn.execute("CREATE INDEX IF NOT EXISTS idx_objects_bucket ON objects(bucket)", [])
            .map_err(Error::DatabaseError)?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_objects_key ON objects(bucket, key)", [])
            .map_err(Error::DatabaseError)?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_objects_hash ON objects(object_hash)", [])
            .map_err(Error::DatabaseError)?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_parts_upload_id ON upload_parts(upload_id)", [])
            .map_err(Error::DatabaseError)?;

        Ok(())
    }

    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}
