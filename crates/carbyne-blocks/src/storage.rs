use rusqlite::{params, Connection};
use std::io::Write;
use thiserror::Error;
use zstd::stream::{read::Decoder, write::Encoder};
use std::io::Read;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Compression error: {0}")]
    Compression(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Session not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, ArchiveError>;

/// Session storage with compression and archive support
pub struct SessionStore {
    db: Connection,
}

impl SessionStore {
    pub fn new(db: Connection) -> Result<Self> {
        // Create tables for hot and archived sessions
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions_hot (
                id TEXT PRIMARY KEY,
                doc_id TEXT NOT NULL,
                content BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                compressed INTEGER NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS sessions_archived (
                id TEXT PRIMARY KEY,
                doc_id TEXT NOT NULL,
                s3_path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                archived_at INTEGER NOT NULL,
                compressed INTEGER NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_sessions_hot_doc ON sessions_hot(doc_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_archived_doc ON sessions_archived(doc_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_hot_updated ON sessions_hot(updated_at);",
        )?;
        Ok(SessionStore { db })
    }

    /// Store session with automatic compression
    pub fn store_session(
        &self,
        session_id: &str,
        doc_id: &str,
        content_json: &str,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Compress content
        let compressed = compress_zstd(content_json)?;

        self.db.execute(
            "INSERT OR REPLACE INTO sessions_hot (id, doc_id, content, created_at, updated_at, compressed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, doc_id, &compressed, now, now, 1],
        )?;

        Ok(())
    }

    /// Retrieve session with automatic decompression
    pub fn get_session(&self, session_id: &str) -> Result<String> {
        // Try hot storage first
        let result = self.db.query_row(
            "SELECT content FROM sessions_hot WHERE id = ?1",
            params![session_id],
            |row| {
                let content: Vec<u8> = row.get(0)?;
                Ok(content)
            },
        );

        match result {
            Ok(content) => decompress_zstd(&content),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Not in hot storage, could check S3 here
                Err(ArchiveError::NotFound)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Archive old sessions to S3 (simulated with path generation)
    pub fn archive_session(&self, session_id: &str) -> Result<String> {
        // Fetch from hot storage
        let content: Vec<u8> = self.db.query_row(
            "SELECT content FROM sessions_hot WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        let doc_id: String = self.db.query_row(
            "SELECT doc_id FROM sessions_hot WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Generate S3 path (caller would upload to this path)
        let s3_path = format!("s3://docs-archive/{}/{}.zst", doc_id, session_id);

        // Move to archived table
        self.db.execute("BEGIN TRANSACTION", [])?;

        self.db.execute(
            "INSERT INTO sessions_archived (id, doc_id, s3_path, created_at, archived_at, compressed)
             SELECT id, doc_id, ?2, created_at, ?1, compressed FROM sessions_hot WHERE id = ?3",
            params![now, &s3_path, session_id],
        )?;

        self.db.execute("DELETE FROM sessions_hot WHERE id = ?1", params![session_id])?;

        self.db.execute("COMMIT", [])?;

        Ok(s3_path)
    }

    /// Archive all sessions older than `days` days
    pub fn archive_old_sessions(&self, days: i64) -> Result<Vec<String>> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            - (days * 24 * 60 * 60 * 1000);

        let mut stmt = self.db.prepare(
            "SELECT id FROM sessions_hot WHERE updated_at < ?1 ORDER BY updated_at ASC",
        )?;

        let ids: Vec<String> = stmt
            .query_map(params![cutoff], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut archived = Vec::new();
        for id in ids {
            archived.push(self.archive_session(&id)?);
        }

        Ok(archived)
    }

    /// Get statistics about storage
    pub fn stats(&self) -> Result<StorageStats> {
        let hot_count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM sessions_hot",
            [],
            |row| row.get(0),
        )?;

        let hot_size: i64 = self.db.query_row(
            "SELECT COALESCE(SUM(LENGTH(content)), 0) FROM sessions_hot",
            [],
            |row| row.get(0),
        )?;

        let archived_count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM sessions_archived",
            [],
            |row| row.get(0),
        )?;

        Ok(StorageStats {
            hot_sessions: hot_count as usize,
            hot_bytes: hot_size as usize,
            archived_sessions: archived_count as usize,
            compression_ratio: 0.35, // Typical for JSON
        })
    }
}

#[derive(Debug, Clone)]
pub struct StorageStats {
    pub hot_sessions: usize,
    pub hot_bytes: usize,
    pub archived_sessions: usize,
    pub compression_ratio: f32,
}

impl StorageStats {
    pub fn hot_size_mb(&self) -> f32 {
        self.hot_bytes as f32 / (1024.0 * 1024.0)
    }

    pub fn estimated_uncompressed_mb(&self) -> f32 {
        self.hot_size_mb() / self.compression_ratio
    }
}

/// Compress data with zstd, level 3 (good compression, fast)
fn compress_zstd(data: &str) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut encoder = Encoder::new(&mut compressed, 3)
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    encoder
        .write_all(data.as_bytes())
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    Ok(compressed)
}

/// Decompress zstd data
fn decompress_zstd(data: &[u8]) -> Result<String> {
    let mut decompressed = Vec::new();
    let mut decoder = Decoder::new(data)
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| ArchiveError::Compression(e.to_string()))?;
    String::from_utf8(decompressed)
        .map_err(|e| ArchiveError::Compression(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() -> Result<()> {
        // Use realistic session JSON (which compresses well)
        let original = r#"{"session_id":"sess_123","doc_id":"doc_456","model":"claude-3-sonnet","messages":[{"role":"user","content":"Can you help me optimize this function?"},{"role":"assistant","content":"Sure! Here are several optimizations you could make...","tokens":245}],"created_at":1699564800000,"updated_at":1699564900000,"usage":{"input_tokens":500,"output_tokens":245}}"#;
        let compressed = compress_zstd(original)?;
        let decompressed = decompress_zstd(&compressed)?;
        assert_eq!(original, decompressed);
        // JSON compresses to ~35% due to repetition
        assert!(compressed.len() < original.len());
        Ok(())
    }

    #[test]
    fn test_session_store() -> Result<()> {
        let db = rusqlite::Connection::open_in_memory()?;
        let store = SessionStore::new(db)?;

        let json = r#"{"msg": "test session"}"#;
        store.store_session("s1", "doc1", json)?;

        let retrieved = store.get_session("s1")?;
        assert_eq!(retrieved, json);

        let stats = store.stats()?;
        assert_eq!(stats.hot_sessions, 1);

        Ok(())
    }

    #[test]
    fn test_archive_old_sessions() -> Result<()> {
        let db = rusqlite::Connection::open_in_memory()?;
        let store = SessionStore::new(db)?;

        // Store a session
        store.store_session("s1", "doc1", r#"{"old": true}"#)?;

        // Archive (simulated - in practice would be older)
        let s3_path = store.archive_session("s1")?;
        assert!(s3_path.contains("s3://docs-archive"));

        // Should not be retrievable from hot storage
        let result = store.get_session("s1");
        assert!(result.is_err());

        let stats = store.stats()?;
        assert_eq!(stats.hot_sessions, 0);
        assert_eq!(stats.archived_sessions, 1);

        Ok(())
    }
}
