use crate::error::{DbError, Result};
use crate::instance::DbId;
use crate::storage::Storage;
use carbyne::{ClientState, Diff, Doc};

/// Snapshot once the tail holds this many diffs.
/// ponytail: fixed count; make it bytes/age when workloads are measured.
pub const SNAPSHOT_EVERY: u64 = 64;

/// Append-only log of committed `Diff`s plus a periodic full snapshot.
///
/// Keys (all prefixed by `db_id`):
/// - `tail:{db}:{seq:020}`  → `Diff::to_bytes()` for one committed change
/// - `snap:{db}`            → `Diff::to_bytes()` of the whole doc at `snap_seq`
/// - `snap:{db}:seq`        → u64 LE, highest seq folded into the snapshot
///
/// `apply` costs one `put`. Snapshot costs one `put` + tail deletes, every
/// `SNAPSHOT_EVERY` applies. Load = snapshot + replay of the tail.
pub struct ChangeLog {
    db_id: DbId,
    storage: Storage,
}

impl ChangeLog {
    pub fn new(db_id: DbId, storage: Storage) -> Self {
        Self { db_id, storage }
    }

    fn tail_prefix(&self) -> String {
        format!("tail:{}:", self.db_id.as_str())
    }

    fn tail_key(&self, seq: u64) -> String {
        format!("{}{:020}", self.tail_prefix(), seq)
    }

    fn snap_key(&self) -> String {
        format!("snap:{}", self.db_id.as_str())
    }

    fn snap_seq_key(&self) -> String {
        format!("snap:{}:seq", self.db_id.as_str())
    }

    /// Append one committed diff at `seq`. One durable write.
    pub fn append(&self, seq: u64, diff: &Diff) -> Result<()> {
        self.storage.put(&self.tail_key(seq), &diff.to_bytes())
    }

    /// Replace the snapshot with `doc`'s full state and drop the tail up to `seq`.
    pub fn snapshot(&self, doc: &Doc, seq: u64) -> Result<()> {
        let full = doc.diff(ClientState::default());
        self.storage.put(&self.snap_key(), &full.to_bytes())?;
        self.storage.put(&self.snap_seq_key(), &seq.to_le_bytes())?;
        for (key, _) in self.storage.iter_keys_with_prefix(&self.tail_prefix())? {
            if Self::seq_of(&key) <= seq {
                self.storage.delete(&key)?;
            }
        }
        Ok(())
    }

    /// Rebuild the doc: snapshot (if any) then every tail diff in order.
    /// Returns the doc and the highest seq applied.
    pub fn load(&self) -> Result<(Doc, u64)> {
        let mut seq = self.snap_seq()?;
        let doc = match self.storage.get(&self.snap_key())? {
            Some(bytes) => {
                let diff = Diff::from_bytes(&bytes).map_err(DbError::SerializationError)?;
                Doc::from(&diff).ok_or_else(|| {
                    DbError::SerializationError("snapshot has no root".into())
                })?
            }
            None => Doc::default(),
        };

        for (key, bytes) in self.storage.iter_keys_with_prefix(&self.tail_prefix())? {
            let diff = Diff::from_bytes(&bytes).map_err(DbError::SerializationError)?;
            doc.apply(&diff);
            seq = seq.max(Self::seq_of(&key));
        }

        Ok((doc, seq))
    }

    /// Number of diffs in the tail (not yet folded into the snapshot).
    pub fn tail_len(&self) -> Result<usize> {
        Ok(self.storage.iter_keys_with_prefix(&self.tail_prefix())?.len())
    }

    fn snap_seq(&self) -> Result<u64> {
        Ok(self
            .storage
            .get(&self.snap_seq_key())?
            .and_then(|b| b.try_into().ok())
            .map(u64::from_le_bytes)
            .unwrap_or(0))
    }

    fn seq_of(key: &str) -> u64 {
        key.rsplit(':').next().and_then(|s| s.parse().ok()).unwrap_or(0)
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
