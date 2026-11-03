use crate::changelog::ChangeLog;
use crate::error::Result;
use crate::storage::Storage;
use carbyne::Doc;
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

// Op type: represents a change to sync across peers (Phase 3.5: now using carbyne::Change)
pub type Op = carbyne::Change;

pub struct DbInstance {
    pub(crate) id: DbId,
    pub(crate) name: DbName,
    pub(crate) doc: Rc<RefCell<Doc>>,
    pub(crate) lamport: Rc<RefCell<u64>>,
    pub(crate) changelog: Arc<ChangeLog>,
}

impl DbInstance {
    pub fn new(id: DbId, name: DbName, changelog: Arc<ChangeLog>) -> Self {
        Self {
            id,
            name,
            doc: Rc::new(RefCell::new(Doc::default())),
            lamport: Rc::new(RefCell::new(0)),
            changelog,
        }
    }

    /// Load instance from persistent storage (snapshot + replay).
    pub fn from_storage(id: DbId, name: DbName, changelog: Arc<ChangeLog>) -> Result<Self> {
        let (doc, _ops, lamport) = changelog.load()?;
        Ok(Self {
            id,
            name,
            doc: Rc::new(RefCell::new(doc)),
            lamport: Rc::new(RefCell::new(lamport)),
            changelog,
        })
    }

    pub fn id(&self) -> DbId {
        self.id
    }

    pub fn name(&self) -> &DbName {
        &self.name
    }

    pub fn apply(&self, ops: Vec<Op>) -> Result<()> {
        // Append ops to changelog first (durability before state change).
        self.changelog.append(&ops)?;

        let mut _doc = self.doc.borrow_mut();
        let mut lamport = self.lamport.borrow_mut();

        // Phase 3.5: Apply real ops to doc (placeholder for now; carbyne::Change merge needs implementation)
        // TODO: merge ops into _doc using carbyne CRDT semantics
        *lamport += 1;

        // Persist lamport checkpoint
        self.changelog.snapshot(&_doc, *lamport)?;

        Ok(())
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
