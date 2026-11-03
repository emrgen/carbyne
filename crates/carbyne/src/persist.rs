use std::cell::RefCell;
use std::default::Default;
use std::rc::{Rc, Weak};

use crate::bimapid::FieldMap;
use crate::change::ChangeStore;
use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::diff::Diff;
use crate::doc::DocId;
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::item::{Content, ItemKind};
use crate::state::ClientState;
use crate::store::{
    DeleteItemStore, DocStore, IdDiff, IdRangeMap, ItemDataStore, PendingStore, ReadyStore,
    WeakStoreRef,
};
use crate::Client;

pub(crate) type StrongStoreDataRef = Rc<RefCell<DocStoreData>>;
pub(crate) type WeakStoreDataRef = Weak<RefCell<DocStoreData>>;

// This is the data structure that will be serialized to disk
#[derive(Debug, Clone, Default)]
pub(crate) struct DocStoreData {
    pub(crate) doc_id: DocId,
    pub(crate) created_by: Client,

    pub(crate) fields: FieldMap,
    pub(crate) id_map: IdRangeMap,
    pub(crate) state: ClientState,

    pub(crate) items: ItemDataStore,
    pub(crate) deleted: DeleteItemStore,

    pub(crate) pending: PendingStore,

    pub(crate) changes: ChangeStore,
}

impl DocStoreData {
    pub(crate) fn from_diff(diff: &Diff) -> DocStoreData {
        let mut doc_store = DocStoreData::default();

        doc_store
    }

    pub(crate) fn diff(&self, state: &ClientState) -> Diff {
        let state = state.as_per(&self.state);

        let items = self.items.diff(&state);

        let deletes = self.deleted.diff(&state);

        let mut clients = self.state.clients.clone();

        for (_, client_id) in clients.iter() {
            if (items.client_size(client_id) + deletes.client_size(client_id)) == 0 {
                // clients.remove(client_id);
            }
        }

        let mut moves = self
            .items
            .iter()
            .any(|(_, store)| store.iter().any(|(_, item)| item.kind == ItemKind::Move));

        Diff::from(
            self.doc_id.clone(),
            self.created_by.clone(),
            self.fields.clone(),
            self.changes.clone(),
            state.clone(),
            items,
            deletes,
        )
    }
}

impl From<DocStore> for DocStoreData {
    fn from(store: DocStore) -> Self {
        let doc_id = store.doc_id;
        let created_by = store.created_by;
        let fields = store.fields.clone();
        let id_map = store.id_map.clone();
        let state = store.state.clone();
        let items = store.items.into();
        let deleted_items = store.deletes.clone();
        let pending = store.pending.clone();
        let changes = store.changes.clone();

        Self {
            doc_id,
            created_by,
            fields,
            id_map,
            state,
            items,
            deleted: deleted_items,
            pending,
            changes,
        }
    }
}

impl Encode for DocStoreData {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        self.doc_id.encode(e, ctx);
        self.created_by.encode(e, ctx);
        self.fields.encode(e, ctx);
        self.state.encode(e, ctx);
        self.items.encode(e, ctx);
        self.deleted.encode(e, ctx);
        self.pending.encode(e, ctx);
        self.changes.encode(e, ctx);
    }
}

impl Decode for DocStoreData {
    fn decode<T: Decoder>(d: &mut T, ctx: &DecodeContext) -> Result<Self, String>
    where
        Self: Sized,
    {
        let doc_id = DocId::decode(d, ctx)?;
        let created_by = Client::decode(d, ctx)?;
        let fields = FieldMap::decode(d, ctx)?;
        let state = ClientState::decode(d, ctx)?;
        let items = ItemDataStore::decode(d, ctx)?;
        let deleted_items = DeleteItemStore::decode(d, ctx)?;
        let pending = PendingStore::decode(d, ctx)?;
        let changes = ChangeStore::decode(d, ctx)?;

        let mut id_map = IdRangeMap::default();
        for (id, item) in items.iter() {
            for (id, item) in item.iter() {
                match &item.content {
                    Content::String(s) => {
                        id_map.insert(id.range(s.len() as u32));
                    }
                    Content::Mark(s) => {
                        id_map.insert(id.range(s.size()));
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            doc_id,
            created_by,
            fields,
            id_map,
            state,
            items,
            deleted: deleted_items,
            pending,
            changes,
        })
    }
}

pub(crate) struct DocStoreDataTransaction {
    pub(crate) doc_id: String,
    pub(crate) store: WeakStoreRef,
    pub(crate) diff: Diff,
    pub(crate) ready: ReadyStore,
    pub(crate) pending_store: PendingStore,
}

impl DocStoreDataTransaction {
    pub(crate) fn new(doc_id: String, store: WeakStoreRef, diff: Diff) -> Self {
        let ready = ReadyStore::default();
        let pending_store = PendingStore::default();

        Self {
            doc_id,
            store,
            diff,
            ready,
            pending_store,
        }
    }

    fn commit(&mut self) {
        self.prepare()
            .and_then(|_| self.merge())
            .and_then(|_| self.apply())
            .unwrap_or_else(|err| {
                log::error!("Tx commit error: {}", err);
                self.rollback();
            })
    }

    fn prepare(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn merge(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn apply(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn rollback(&mut self) {}
}
