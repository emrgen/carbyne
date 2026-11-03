use lru::LruCache;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub use carbyne::{Doc, Type, ClientState};


pub mod storage;
pub mod archive_job;
pub mod s3_backend;

// Re-export for convenience
pub use storage::SessionStore;
pub use archive_job::{ArchiveJob, ArchiveConfig};
pub use s3_backend::{S3Backend, S3Config};

#[derive(Error, Debug)]
pub enum BlockError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("block not found: {0}")]
    BlockNotFound(usize),
    #[error("invalid block structure")]
    InvalidBlock,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = BlockError> = std::result::Result<T, E>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub block_type: String,
    pub content: String,
    pub version: Vec<u8>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct BlockChange {
    pub block_id: usize,
    pub is_new: bool,
    pub is_deleted: bool,
    pub block: Option<Block>,
}

pub struct BlockStore {
    doc: Arc<Doc>,
    db: Mutex<Connection>,
    cache: Mutex<LruCache<usize, Block>>,
}

impl BlockStore {
    pub fn new<P: AsRef<Path>>(doc: Arc<Doc>, db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER,
                block_type TEXT NOT NULL,
                content TEXT NOT NULL,
                version BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_parent_id ON blocks(parent_id);
            CREATE INDEX IF NOT EXISTS idx_updated_at ON blocks(updated_at);",
        )?;

        Ok(BlockStore {
            doc,
            db: Mutex::new(conn),
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
        })
    }

    pub fn sync(&self, version_from: ClientState) -> Result<Vec<BlockChange>> {
        let dirty_indices = self.doc.dirty_blocks(version_from);
        let root = self.doc.list();
        let current_version = self.doc.version();

        let mut changes = Vec::new();
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for block_id in dirty_indices {
            if let Some(block_type) = root.get(block_id) {
                let block = self.rebuild_block(block_id, &block_type, &current_version, now)?;
                let exists: bool = tx
                    .query_row("SELECT 1 FROM blocks WHERE id = ?", [block_id], |_| Ok(true))
                    .optional()?
                    .unwrap_or(false);

                if exists {
                    tx.execute(
                        "UPDATE blocks SET parent_id = ?, block_type = ?, content = ?, version = ?, updated_at = ? WHERE id = ?",
                        params![block.parent_id, &block.block_type, &block.content, &block.version, now, block_id],
                    )?;
                } else {
                    tx.execute(
                        "INSERT INTO blocks (id, parent_id, block_type, content, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                        params![block_id, block.parent_id, &block.block_type, &block.content, &block.version, now, now],
                    )?;
                }

                changes.push(BlockChange {
                    block_id,
                    is_new: !exists,
                    is_deleted: false,
                    block: Some(block),
                });

                self.cache.lock().unwrap().pop(&block_id);
            }
        }

        tx.commit()?;
        Ok(changes)
    }

    pub fn get_block(&self, id: usize) -> Result<Block> {
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(block) = cache.get(&id) {
                return Ok(block.clone());
            }
        }

        let db = self.db.lock().unwrap();
        let block = db
            .query_row(
                "SELECT id, parent_id, block_type, content, version, created_at, updated_at FROM blocks WHERE id = ?",
                [id],
                |row| {
                    Ok(Block {
                        id: row.get(0)?,
                        parent_id: row.get(1)?,
                        block_type: row.get(2)?,
                        content: row.get(3)?,
                        version: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                },
            )
            .optional()?
            .ok_or(BlockError::BlockNotFound(id))?;

        self.cache.lock().unwrap().put(id, block.clone());
        Ok(block)
    }

    pub fn get_blocks(&self, ids: &[usize]) -> Result<Vec<Block>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let db = self.db.lock().unwrap();
        let placeholders = vec!["?"; ids.len()].join(",");
        let query = format!(
            "SELECT id, parent_id, block_type, content, version, created_at, updated_at FROM blocks WHERE id IN ({}) ORDER BY id",
            placeholders
        );

        let mut stmt = db.prepare(&query)?;
        let blocks = stmt
            .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
                Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    block_type: row.get(2)?,
                    content: row.get(3)?,
                    version: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(blocks)
    }

    pub fn get_children(&self, parent_id: usize) -> Result<Vec<usize>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT id FROM blocks WHERE parent_id = ? ORDER BY id")?;
        let children = stmt
            .query_map([parent_id], |row| row.get::<_, usize>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(children)
    }

    pub fn all_blocks(&self) -> Result<Vec<Block>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, parent_id, block_type, content, version, created_at, updated_at FROM blocks ORDER BY id"
        )?;
        let blocks = stmt
            .query_map([], |row| {
                Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    block_type: row.get(2)?,
                    content: row.get(3)?,
                    version: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(blocks)
    }

    fn rebuild_block(&self, block_id: usize, block_type: &Type, version: &ClientState, now: i64) -> Result<Block> {
        let (block_type_str, content, parent_id) = match block_type {
            Type::Map(m) => {
                let bt = "block".to_string();
                let content = m
                    .get("title")
                    .and_then(|t| Self::extract_text(&t))
                    .or_else(|| m.get("content").and_then(|t| Self::extract_text(&t)))
                    .unwrap_or_default();
                (bt, content, None)
            }
            _ => ("unknown".to_string(), String::new(), None),
        };

        Ok(Block {
            id: block_id,
            parent_id,
            block_type: block_type_str,
            content,
            version: bincode::serialize(version).unwrap_or_default(),
            created_at: now,
            updated_at: now,
        })
    }

    fn extract_text(t: &Type) -> Option<String> {
        match t {
            Type::Text(_) => Some("[text]".to_string()),
            Type::String(_) => Some("[string]".to_string()),
            _ => None,
        }
    }

    pub fn rebuild_all(&self) -> Result<()> {
        let root = self.doc.list();
        let current_version = self.doc.version();
        let mut db = self.db.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute("DELETE FROM blocks", [])?;
        let tx = db.transaction()?;

        for i in 0..root.size() {
            if let Some(block_type) = root.get(i as usize) {
                let block = self.rebuild_block(i as usize, &block_type, &current_version, now)?;
                tx.execute(
                    "INSERT INTO blocks (id, parent_id, block_type, content, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
                    params![block.id, block.parent_id, &block.block_type, &block.content, &block.version, block.created_at, block.updated_at],
                )?;
            }
        }

        tx.commit()?;
        self.cache.lock().unwrap().clear();
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        db.execute("DELETE FROM blocks", [])?;
        self.cache.lock().unwrap().clear();
        Ok(())
    }
}


