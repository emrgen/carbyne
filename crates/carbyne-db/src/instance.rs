use crate::changelog::ChangeLog;
use crate::error::Result;
use carbyne::{Doc, Diff, ClientState};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use uuid::Uuid;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DbId(pub Uuid);

impl DbId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for DbId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DbName(pub String);

// Phase 4: Op type = Diff (use existing carbyne merge semantics).
// Note: Change was exported but carbyne doesn't provide Change merge API yet.
// Diffs are state snapshots; less efficient but compatible with Doc.apply().
pub type Op = Diff;

pub struct DbInstance {
    pub(crate) id: DbId,
    pub(crate) name: DbName,
    pub(crate) doc: Rc<RefCell<Doc>>,
    pub(crate) lamport: Rc<RefCell<u64>>,
    pub(crate) changelog: Arc<ChangeLog>,
}

impl DbInstance {
    pub fn new(id: DbId, name: DbName, changelog: Arc<ChangeLog>) -> Result<Self> {
        Ok(Self {
            id,
            name,
            doc: Rc::new(RefCell::new(Doc::default())),
            lamport: Rc::new(RefCell::new(0)),
            changelog,
        })
    }

    /// Load instance from persistent storage (snapshot + replay).
    pub fn from_storage(id: DbId, name: DbName, changelog: Arc<ChangeLog>) -> Result<Self> {
        let (doc, lamport) = changelog.load()?;
        Ok(Self {
            id,
            name,
            doc: Rc::new(RefCell::new(doc)),
            lamport: Rc::new(RefCell::new(lamport)),
            changelog,
        })
    }

    /// Create from a snapshot (diff from peer).
    pub fn from_snapshot(_id: DbId, name: DbName, changelog: Arc<ChangeLog>, _snapshot: &Diff) -> Result<Self> {
        // TODO: Apply snapshot to initial doc state
        Self::new(_id, name, changelog)
    }

    pub fn id(&self) -> DbId {
        self.id
    }

    pub fn name(&self) -> &DbName {
        &self.name
    }

    /// Current version of the document (for sync protocol).
    pub fn version(&self) -> ClientState {
        self.doc.borrow().version()
    }

    /// Apply incoming diffs from peers (Phase 4: real CRDT merge via Diff).
    /// Stores in changelog first (durability), then merges into doc.
    pub fn apply(&self, diffs: Vec<Op>) -> Result<()> {
        if diffs.is_empty() {
            return Ok(());
        }

        let mut doc = self.doc.borrow_mut();
        let mut lamport = self.lamport.borrow_mut();

        // Apply each diff to local doc (CRDT merges)
        for diff in &diffs {
            doc.apply(diff);
        }
        *lamport += diffs.len() as u64;

        Ok(())
    }

    /// Get all diffs since a given version (for sync response).
    pub fn diffs_since(&self, since: ClientState) -> Op {
        self.doc.borrow().diff(since)
    }

    pub fn doc(&self) -> std::cell::Ref<Doc> {
        self.doc.borrow()
    }

    pub fn lamport(&self) -> u64 {
        *self.lamport.borrow()
    }

    pub fn changelog(&self) -> Arc<ChangeLog> {
        Arc::clone(&self.changelog)
    }
}

impl Clone for DbInstance {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            doc: Rc::clone(&self.doc),
            lamport: Rc::clone(&self.lamport),
            changelog: Arc::clone(&self.changelog),
        }
    }
}
