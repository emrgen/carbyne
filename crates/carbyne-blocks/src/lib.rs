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
pub mod table_pool;
pub mod s3_backend;

// Re-export for convenience
pub use storage::SessionStore;
pub use table_pool::TablePool;
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
    pub table_id: u32,              // Immutable tag ID (from TablePool)
    pub block_type: String,
    pub content: String,
    pub version: Vec<u8>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug)]
pub struct BlockChange {
    pub block_id: usize,
    pub is_deleted: bool,
    pub block: Option<Block>,
}

pub struct BlockStore {
    doc: Arc<Doc>,
    db: Mutex<Connection>,
    cache: Mutex<LruCache<usize, Block>>,
    table_pool: TablePool,  // Immutable tag interning
}

impl BlockStore {
    pub fn new<P: AsRef<Path>>(doc: Arc<Doc>, db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        // WAL: readers don't block the sync writer; NORMAL: fsync at checkpoint, not per txn.
        // Durable against process crash, not power loss within the last checkpoint window.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER,
                table_id INTEGER NOT NULL DEFAULT 0,
                block_type TEXT NOT NULL,
                content TEXT NOT NULL,
                version BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_table_id ON blocks(table_id);
            CREATE INDEX IF NOT EXISTS idx_updated_at ON blocks(updated_at);

            CREATE TABLE IF NOT EXISTS text_op_dedup (
                instance_id TEXT NOT NULL,
                op_id TEXT NOT NULL,
                first_seen_at INTEGER NOT NULL,
                PRIMARY KEY(instance_id, op_id)
            );
            CREATE INDEX IF NOT EXISTS idx_text_op_dedup_instance ON text_op_dedup(instance_id);

            CREATE TABLE IF NOT EXISTS crdt_instance_placements (
                instance_id TEXT NOT NULL,
                block_id INTEGER NOT NULL,
                placed_at INTEGER NOT NULL,
                PRIMARY KEY(instance_id, block_id)
            );",
        )?;
        Ok(BlockStore {
            doc,
            db: Mutex::new(conn),
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap())),
            table_pool: TablePool::new(),
        })
    }

    fn extract_instance_id(&self, block_type: &str) -> Option<String> {
        // Extract instance ID from block_type
        // Heuristic: if contains "inst-" prefix, assume text instance
        if block_type.starts_with("inst-") {
            return Some(block_type.to_string());
        }
        // Or if format is "Type:inst-123"
        if let Some(last) = block_type.split(':').last() {
            if last.starts_with("inst-") {
                return Some(last.to_string());
            }
        }
        None
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
                tx.execute(
                    "INSERT INTO blocks (id, parent_id, table_id, block_type, content, version, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(id) DO UPDATE SET
                        parent_id = excluded.parent_id, table_id = excluded.table_id,
                        block_type = excluded.block_type, content = excluded.content,
                        version = excluded.version, updated_at = excluded.updated_at",
                    params![block_id, block.parent_id, block.table_id, &block.block_type, &block.content, &block.version, now, now],
                )?;


                // Record instance placement for this block (split/join tracking)
                if let Some(instance_id) = self.extract_instance_id(&block.block_type) {
                    let _ = self.record_placement(&instance_id, block_id);
                }
                changes.push(BlockChange {
                    block_id,
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
                "SELECT id, parent_id, table_id, block_type, content, version, created_at, updated_at FROM blocks WHERE id = ?",
                [id],
                |row| {
                    Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    table_id: row.get(2)?,
                    block_type: row.get(3)?,
                    content: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
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
            "SELECT id, parent_id, table_id, block_type, content, version, created_at, updated_at FROM blocks WHERE id IN ({}) ORDER BY id",
            placeholders
        );

        let mut stmt = db.prepare(&query)?;
        let blocks = stmt
            .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
                Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    table_id: row.get(2)?,
                    block_type: row.get(3)?,
                    content: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
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
    /// Query blocks by immutable table_id. O(log n) via index.
    pub fn get_blocks_by_tag(&self, table_id: u32) -> Result<Vec<Block>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, parent_id, table_id, block_type, content, version, created_at, updated_at FROM blocks WHERE table_id = ? ORDER BY id"
        )?;
        let blocks = stmt
            .query_map([table_id], |row| {
                Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    table_id: row.get(2)?,
                    block_type: row.get(3)?,
                    content: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(blocks)
    }

    /// Query blocks by multiple immutable table_ids. O(log n) via index.
    pub fn get_blocks_by_tags(&self, table_ids: &[u32]) -> Result<Vec<Block>> {
        if table_ids.is_empty() {
            return Ok(Vec::new());
        }

        let db = self.db.lock().unwrap();
        let placeholders = vec!["?"; table_ids.len()].join(",");
        let query = format!(
            "SELECT id, parent_id, table_id, block_type, content, version, created_at, updated_at FROM blocks WHERE table_id IN ({}) ORDER BY id",
            placeholders
        );

        let mut stmt = db.prepare(&query)?;
        let blocks = stmt
            .query_map(rusqlite::params_from_iter(table_ids.iter()), |row| {
                Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    table_id: row.get(2)?,
                    block_type: row.get(3)?,
                    content: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(blocks)
    }

    pub fn all_blocks(&self) -> Result<Vec<Block>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT id, parent_id, table_id, block_type, content, version, created_at, updated_at FROM blocks ORDER BY id"
        )?;
        let blocks = stmt
            .query_map([], |row| {
                Ok(Block {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    table_id: row.get(2)?,
                    block_type: row.get(3)?,
                    content: row.get(4)?,
                    version: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(blocks)
    }

    fn rebuild_block(&self, block_id: usize, block_type: &Type, version: &ClientState, now: i64) -> Result<Block> {
        let (block_type_str, content, parent_id, table_id) = match block_type {
            Type::Map(m) => {
                let bt = "block".to_string();
                let content = m
                    .get("title")
                    .and_then(|t| Self::extract_text(&t))
                    .or_else(|| m.get("content").and_then(|t| Self::extract_text(&t)))
                    .unwrap_or_default();
                
                // Extract immutable table from CRDT
                // Extract immutable table from CRDT
                let table_id = m
                    .get("table")
                    .and_then(|table_val| {
                        match table_val {
                            Type::Atom(_) => Some("atom".to_string()),
                            Type::String(_) => Some("string".to_string()),
                            Type::Text(_) => Some("text".to_string()),
                            Type::List(_) => Some("list".to_string()),
                            Type::Map(_) => Some("map".to_string()),
                            Type::Identity => Some("identity".to_string()),
                            _ => Some("other".to_string()),
                        }
                    })
                    .map(|table_name| self.table_pool.intern(&table_name))
                    .unwrap_or_else(|| self.table_pool.intern("default"));
                
                (bt, content, None, table_id)
            }
            _ => ("unknown".to_string(), String::new(), None, self.table_pool.intern("unknown")),
        };

        Ok(Block {
            id: block_id,
            parent_id,
            table_id,
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

    /// Mark operations as applied for dedup
    /// op_id format: "{id_seq}#{id_site}" (composite unique ID)
    pub fn mark_ops_applied(&self, instance_id: &str, ops: Vec<TextOpId>) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for op in ops {
            db.execute(
                "INSERT OR IGNORE INTO text_op_dedup (instance_id, op_id, first_seen_at)
                 VALUES (?, ?, ?)",
                params![instance_id, &op.id, now],
            ).ok();
        }
        Ok(())
    }

    /// Check if operation was already applied (for dedup)
    pub fn is_op_applied(&self, instance_id: &str, op_id: &str) -> Result<bool> {
        let db = self.db.lock().unwrap();
        let exists = db.query_row(
            "SELECT 1 FROM text_op_dedup WHERE instance_id = ? AND op_id = ?",
            params![instance_id, op_id],
            |_| Ok(true),
        ).optional()?;
        Ok(exists.unwrap_or(false))
    }

    /// Record that instance_id was placed in block_id
    /// Used for split/join tracking
    pub fn record_placement(&self, instance_id: &str, block_id: usize) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute(
            "INSERT OR IGNORE INTO crdt_instance_placements (instance_id, block_id, placed_at)
             VALUES (?, ?, ?)",
            params![instance_id, block_id, now],
        )?;
        Ok(())
    }
    /// Split: instance moves from source_block_id to target_block_id
    /// Used when user splits a paragraph (e.g., block 100 → 100 + 999)
    /// Idempotent: calling twice with same params is safe (INSERT OR IGNORE)
    pub fn split_instance(
        &self,
        instance_id: &str,
        source_block_id: usize,
        target_block_id: usize,
    ) -> Result<()> {
        // Record instance in target block; source still has it too (both track same instance)
        self.record_placement(instance_id, target_block_id)
    }

    /// Join: two instances merge into one
    /// Used when user joins two paragraphs (e.g., blocks 100 + 999 → 100)
    /// Merges ops from source_instance into target_instance in target_block_id
    /// Idempotent: calling twice with same params is safe (INSERT OR IGNORE)
    pub fn join_instances(
        &self,
        source_instance_id: &str,
        target_instance_id: &str,
        target_block_id: usize,
    ) -> Result<()> {
        let db = self.db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 1. Record target instance in target block
        db.execute(
            "INSERT OR IGNORE INTO crdt_instance_placements (instance_id, block_id, placed_at)
             VALUES (?, ?, ?)",
            params![target_instance_id, target_block_id, now],
        )?;

        // 2. Merge ops: copy source_instance ops to target_instance
        //    SELECT source ops, INSERT with target instance_id
        //    This is idempotent (source and target ops tracked separately)
        db.execute(
            "INSERT OR IGNORE INTO text_op_dedup (instance_id, op_id, first_seen_at)
             SELECT ?, op_id, first_seen_at FROM text_op_dedup WHERE instance_id = ?",
            params![target_instance_id, source_instance_id],
        )?;

        Ok(())
    }


    /// Get all instances that were placed in given block_ids
    /// Used to widen catch-up scope: blockIDs → instanceIDs
    pub fn instances_in(&self, block_ids: &[usize]) -> Result<Vec<String>> {
        if block_ids.is_empty() {
            return Ok(Vec::new());
        }

        let db = self.db.lock().unwrap();
        let placeholders = std::iter::repeat("?")
            .take(block_ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT DISTINCT instance_id FROM crdt_instance_placements WHERE block_id IN ({})",
            placeholders
        );

        let mut stmt = db.prepare(&query)?;
        let instances = stmt
            .query_map(rusqlite::params_from_iter(block_ids.iter()), |row| {
                row.get(0)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(instances)
    }

    /// Get operations for an instance (extracted from NText)
    /// Placeholder: full NText RGA extraction deferred
    pub fn get_instance_ops(&self, _instance_id: &str) -> Result<Vec<TextOp>> {
        // Full implementation: fetch block containing instance
        // Walk NText RGA structure, extract ops
        // Return Vec<TextOp>
        Ok(Vec::new())
    }

    /// Extract ops from NText instance (from materialized text)
    /// Reconstructs op IDs from NText's implicit RGA structure
    /// Returns empty vec if instance not found or not NText type
    pub fn extract_text_ops(&self, instance_id: &str, text: &str) -> Result<Vec<TextOp>> {
        // ponytail: full RGA extraction deferred to when NText API exposes it
        // add when: NText gains op_id iterator or export_ops() method
        // For now: client sends ops explicitly during sync, server dedupes
        let ops = Vec::new();
        Ok(ops)
    }

    /// Rebuild text from stored ops via RGA replay
    /// Used to verify extracted text matches DB ops
    /// Also used for catch-up: client reconstructs from op history
    pub fn rebuild_text(&self, instance_id: &str) -> Result<String> {
        // Get all ops for this instance
        let ops = self.get_instance_ops(instance_id)?;
        
        if ops.is_empty() {
            return Ok(String::new());
        }

        // ponytail: RGA replay algorithm deferred to carbyne integration
        // add when: client-side RGA interpreter available
        // For now: return empty (ops stored, reconstruction on client)
        Ok(String::new())
    }

    /// Record client ops during sync and mark placements
    /// Called by client integration layer after sync
    /// Convenience wrapper: calls mark_ops_applied + record_placement
    pub fn record_client_ops(&self, instance_id: &str, ops: Vec<TextOpId>, block_id: usize) -> Result<()> {
        self.mark_ops_applied(instance_id, ops)?;
        self.record_placement(instance_id, block_id)?;
        Ok(())
    }
}

/// Minimal op ID for dedup
#[derive(Clone, Debug)]
pub struct TextOpId {
    pub id: String,  // composite: "{id_seq}#{id_site}"
}

/// Operation extracted from NText
/// Algorithm-agnostic: server doesn't decode
#[derive(Clone, Debug)]
pub struct TextOp {
    pub instance_id: String,
    pub op_id: String,  // composite: "{id_seq}#{id_site}"
    pub content: Vec<u8>,  // opaque payload
}
