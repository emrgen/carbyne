use crate::error::{DbError, Result};
use crate::instance::DbId;
use crate::storage::Storage;
use carbyne::Doc;
use serde::{Deserialize, Serialize};

// Phase 2: ops are placeholder; Phase 3 replaces with real carbyne::Change
pub type Op = ();

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpsList {
    pub ops: Vec<Op>,
}

pub struct ChangeLog {
    db_id: DbId,
    storage: Storage,
}

impl ChangeLog {
    pub fn new(db_id: DbId, storage: Storage) -> Self {
        Self { db_id, storage }
    }

    fn ops_key(&self) -> String {
        format!("changelog:{}:ops", self.db_id.as_str())
    }

    fn lamport_key(&self) -> String {
        format!("snapshot:{}:lamport", self.db_id.as_str())
    }

    /// Append ops to changelog.
    pub fn append(&self, ops: &[Op]) -> Result<()> {
        let mut stored = self.load_ops()?;
        stored.extend_from_slice(ops);

        let list = OpsList { ops: stored };
        let serialized = bincode::serialize(&list)
            .map_err(|e| DbError::SerializationError(e.to_string()))?;

        self.storage.put(&self.ops_key(), &serialized)
    }

    /// Load ops post-snapshot.
    /// Phase 2: returns default Doc; Phase 3 will load snapshot.
    pub fn load(&self) -> Result<(Doc, Vec<Op>, u64)> {
        let doc = Doc::default();
        let ops = self.load_ops()?;
        let lamport = self.load_lamport()?;

        Ok((doc, ops, lamport))
    }

    /// Save lamport at current snapshot.
    /// Phase 2: skips Doc serialization; Phase 3 adds it.
    pub fn snapshot(&self, _doc: &Doc, lamport: u64) -> Result<()> {
        self.storage.put(&self.lamport_key(), &lamport.to_le_bytes())?;
        // Clear ops log after snapshot
        self.storage.delete(&self.ops_key())?;
        Ok(())
    }

    /// Count ops in changelog.
    pub fn ops_count(&self) -> Result<usize> {
        Ok(self.load_ops()?.len())
    }

    fn load_ops(&self) -> Result<Vec<Op>> {
        match self.storage.get(&self.ops_key())? {
            Some(bytes) => {
                let list: OpsList = bincode::deserialize(&bytes)
                    .map_err(|e| DbError::SerializationError(e.to_string()))?;
                Ok(list.ops)
            }
            None => Ok(Vec::new()),
        }
    }

    fn load_lamport(&self) -> Result<u64> {
        match self.storage.get(&self.lamport_key())? {
            Some(bytes_vec) => {
                if bytes_vec.len() == 8 {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&bytes_vec);
                    Ok(u64::from_le_bytes(arr))
                } else {
                    Ok(0)
                }
            }
            None => Ok(0),
        }
    }
}

impl Clone for ChangeLog {
    fn clone(&self) -> Self {
        Self {
            db_id: self.db_id,
            storage: self.storage.clone(),
        }
    }
}
