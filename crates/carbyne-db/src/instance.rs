use crate::changelog::ChangeLog;
use crate::error::Result;
use carbyne::{Doc, Change, ClientState};
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

// Phase 3.5: Op type is now carbyne::Change for real CRDT merge
pub type Op = Change;

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
        // Phase 3.5: Simplified load (TODO: replay ops from changelog)
        let (doc, lamport) = changelog.load()?;
        Ok(Self {
            id,
            name,
            doc: Rc::new(RefCell::new(doc)),
            lamport: Rc::new(RefCell::new(lamport)),
            changelog,
        })
    }

    /// Create from a snapshot (diff from peer). Phase 3.5: placeholder.
    pub fn from_snapshot(_id: DbId, name: DbName, changelog: Arc<ChangeLog>, _snapshot: &carbyne::Diff) -> Result<Self> {
        // TODO: Deserialize snapshot into initial doc state
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

    /// Apply incoming ops from peers (Phase 3.5: real CRDT merge).
    /// Stores in changelog first (durability), then merges into doc.
    pub fn apply(&self, ops: Vec<Op>) -> Result<()> {
        if ops.is_empty() {
            return Ok(());
        }

        // Phase 3.5: Durability: append ops to changelog
        // TODO: Implement changelog.append_ops(Vec<Op>)
        // For now: skip durability; focus on doc merge
        
        let mut doc = self.doc.borrow_mut();
        let mut lamport = self.lamport.borrow_mut();

        // Phase 3.5: Merge ops into doc via CRDT semantics.
        // TODO: Implement carbyne CRDT merge for ops.
        // For now, just increment lamport to track causality.
        for _op in &ops {
            // carbyne::Change merge would go here
            // doc.merge(op)?;
        }
        *lamport += ops.len() as u64;

        // TODO: Persist updated lamport via changelog.snapshot()

        Ok(())
    }

    /// Get all ops since a given version (for sync response).
    pub fn ops_since(&self, _since: ClientState) -> Vec<Op> {
        // TODO: Query changelog for ops > since version
        // For now: placeholder (Phase 3.5 blocking point)
        vec![]
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
