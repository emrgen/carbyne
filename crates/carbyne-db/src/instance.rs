use crate::changelog::{ChangeLog, SNAPSHOT_EVERY};
use crate::error::Result;
use carbyne::{ClientState, Diff, Doc};
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

pub struct DbInstance {
    pub(crate) id: DbId,
    pub(crate) name: DbName,
    pub(crate) doc: Rc<RefCell<Doc>>,
    /// local append sequence; only orders the tail, carries no causal meaning
    pub(crate) seq: Rc<RefCell<u64>>,
    /// version as of the last persisted diff; `commit()` diffs from here
    pub(crate) persisted: Rc<RefCell<ClientState>>,
    pub(crate) changelog: Arc<ChangeLog>,
}

impl DbInstance {
    /// Create a fresh instance. Persists a genesis snapshot so a later `load`
    /// rebuilds the same root/client identity the tail diffs reference.
    pub fn new(id: DbId, name: DbName, changelog: Arc<ChangeLog>) -> Result<Self> {
        Self::genesis(id, name, changelog, Doc::default())
    }

    /// Create an instance seeded from a peer's full diff; a new client id is
    /// assigned so local edits do not collide with the peer's.
    pub fn from_snapshot(
        id: DbId,
        name: DbName,
        changelog: Arc<ChangeLog>,
        snapshot: &Diff,
    ) -> Result<Self> {
        let doc = Doc::from(snapshot)
            .ok_or_else(|| crate::error::DbError::SerializationError("snapshot has no root".into()))?;
        doc.update_client();
        Self::genesis(id, name, changelog, doc)
    }

    fn genesis(id: DbId, name: DbName, changelog: Arc<ChangeLog>, doc: Doc) -> Result<Self> {
        changelog.snapshot(&doc, 0)?;
        let persisted = Rc::new(RefCell::new(doc.version()));
        Ok(Self {
            id,
            name,
            doc: Rc::new(RefCell::new(doc)),
            seq: Rc::new(RefCell::new(0)),
            persisted,
            changelog,
        })
    }

    /// Load instance from persistent storage (snapshot + tail replay).
    pub fn from_storage(id: DbId, name: DbName, changelog: Arc<ChangeLog>) -> Result<Self> {
        let (doc, seq) = changelog.load()?;
        let persisted = Rc::new(RefCell::new(doc.version()));
        Ok(Self {
            id,
            name,
            doc: Rc::new(RefCell::new(doc)),
            seq: Rc::new(RefCell::new(seq)),
            persisted,
            changelog,
        })
    }

    pub fn id(&self) -> DbId {
        self.id
    }

    pub fn name(&self) -> &DbName {
        &self.name
    }

    /// Integrate a remote diff: persist it, then apply it.
    /// Durability before visibility: the tail write lands before `doc` changes.
    pub fn apply(&self, diff: &Diff) -> Result<()> {
        self.record(diff, |doc| doc.apply(diff))
    }

    /// Persist local edits made directly on `doc()`: commit, then append the diff.
    pub fn commit(&self) -> Result<()> {
        self.doc.borrow().commit();
        let since = self.persisted.borrow().clone();
        let diff = self.doc.borrow().diff(since);
        if diff.is_empty() {
            return Ok(());
        }
        self.record(&diff, |_| {})
    }

    fn record(&self, diff: &Diff, mutate: impl FnOnce(&Doc)) -> Result<()> {
        let mut seq = self.seq.borrow_mut();
        let next = *seq + 1;
        self.changelog.append(next, diff)?;
        let doc = self.doc.borrow();
        mutate(&doc);
        *seq = next;
        *self.persisted.borrow_mut() = doc.version();
        if next % SNAPSHOT_EVERY == 0 {
            self.changelog.snapshot(&doc, next)?;
        }
        Ok(())
    }

    /// Diff of everything this instance has that `since` does not.
    pub fn diff_since(&self, since: ClientState) -> Diff {
        self.doc.borrow().diff(since)
    }

    pub fn doc(&self) -> std::cell::Ref<Doc> {
        self.doc.borrow()
    }

    /// Causal version: per-client vector clock. This is what peers exchange.
    pub fn version(&self) -> ClientState {
        self.doc.borrow().version()
    }

    /// Local append sequence (tail position). Not comparable across instances.
    pub fn seq(&self) -> u64 {
        *self.seq.borrow()
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
            seq: Rc::clone(&self.seq),
            persisted: Rc::clone(&self.persisted),
            changelog: Arc::clone(&self.changelog),
        }
    }
}
