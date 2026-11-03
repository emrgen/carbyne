use crate::bimapid::{ClientId, ClientMapper, FieldMap};
use crate::change::{ChangeData, ChangeId, ChangeStore};
use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::doc::DocId;
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, WithId, WithIdRange};
use crate::item::{ItemData, Optimize};
use crate::state::ClientState;
use crate::store::{DeleteItemStore, DocStore, IdDiff, ItemDataStore, ItemStore};
use crate::Client;
use hashbrown::{HashMap, HashSet};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_columnar::Itertools;
use std::cell::RefMut;
use std::cmp::max;
use std::ops::Add;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Diff {
    pub created_by: Client,
    pub doc_id: DocId,
    pub fields: FieldMap,
    pub state: ClientState,
    pub changes: ChangeStore,
    pub items: ItemDataStore,
    pub deletes: DeleteItemStore,
}

impl Diff {
    pub(crate) fn has_root(&self) -> bool {
        self.get_root().is_some()
    }

    pub(crate) fn get_root(&self) -> Option<ItemData> {
        let client = self.state.clients.get_client_id(&self.created_by)?;
        self.items.get(&Id::new(*client, 1))
    }
}

impl Diff {
    /// create a diff with the given doc_id and created_by
    pub(crate) fn new(doc_id: DocId, created_by: Client) -> Diff {
        Self {
            doc_id,
            created_by,
            ..Default::default()
        }
    }

    /// create a diff from the given parameters
    pub(crate) fn from(
        doc_id: DocId,
        created_by: Client,
        fields: FieldMap,
        changes: ChangeStore,
        state: ClientState,
        items: ItemDataStore,
        deletes: DeleteItemStore,
    ) -> Diff {
        Diff {
            created_by,
            doc_id,
            fields,
            state,
            changes,
            items,
            deletes,
        }
    }

    /// get all the changes for this diff
    ///
    pub(crate) fn changes(&self) -> (HashMap<ChangeId, ChangeData>, HashSet<ChangeId>) {
        let mut non_mover_changes = HashMap::default();
        let mut mover_changes = HashSet::new();

        let mut clients = HashSet::new();
        clients.extend(self.items.clients());
        clients.extend(self.deletes.clients());

        if self.changes.size() == 0 {
            // panic!("no changes in diff, using items and deletes");
            for client in clients {
                let mut items = ItemStore::default();
                let mut delete_items = DeleteItemStore::default();
                let mut moves = false;
                let mut min_tick = u32::MAX;
                let mut max_tick = u32::MIN;

                if let Some(store) = self.items.id_store(&client) {
                    for (_, item) in store.iter() {
                        moves |= item.kind.is_move();
                        items.insert(item.clone());
                        let range = item.range();
                        min_tick = min_tick.min(range.start);
                        max_tick = max_tick.max(range.end);
                    }
                }

                if let Some(store) = self.deletes.id_store(&client) {
                    for (_, item) in store.iter() {
                        let range = item.range();
                        min_tick = min_tick.min(range.start);
                        max_tick = max_tick.max(range.end);
                    }
                }

                if min_tick != u32::MAX && max_tick != u32::MIN {
                    // let change = ChangeData::new(
                    //     ChangeId::new(client, min_tick, max_tick),
                    //     items.into(),
                    //     delete_items.into(),
                    // );
                    // mover_changes.push(change.clone());
                    // changes.add(change);
                }
            }
        } else {
            // panic!("changes in diff, using changes");
            // if there are changes, we need to get the changes for each client
            for (client_id, change_store) in self.changes.iter() {
                for (change_id) in change_store.iter() {
                    // println!("### change_id: {:?}", change_id);
                    let mut items = Vec::new();
                    let mut delete_items = Vec::new();
                    let mut moves = false;

                    if let Some(store) = self.items.id_store(client_id) {
                        // println!("@@@ item: {:?}", IdRange::from(change_id.clone()));
                        for item in store.get_range(&change_id.clone().into()) {
                            // println!("### item: {:?}", item);
                            moves |= item.kind.is_move();
                            items.push(item.clone());
                        }
                    }

                    if let Some(store) = self.deletes.id_store(client_id) {
                        for item in store.get_range(&change_id.clone().into()) {
                            delete_items.push(item.clone());
                        }
                    }

                    let change =
                        ChangeData::new(change_id.clone(), items, delete_items).with_mover(moves);

                    // println!("change_id: {:?}, change: {:?}\n", change_id, change);
                    if moves {
                        mover_changes.insert(change.id);
                    }

                    non_mover_changes.insert(change.id, change);
                }
            }
        }

        (non_mover_changes, mover_changes)
    }

    // create a diff from a diff
    pub fn diff(&self, state: &ClientState) -> Diff {
        Diff {
            doc_id: self.doc_id.clone(),
            created_by: self.created_by.clone(),
            fields: self.fields.clone(),
            state: self.state.clone(),
            changes: self.changes.clone(),
            items: self.items.diff(state),
            deletes: self.deletes.diff(state),
        }
    }

    pub(crate) fn from_deleted_items(deleted_items: DeleteItemStore) -> Diff {
        Diff {
            deletes: deleted_items,
            ..Default::default()
        }
    }

    pub(crate) fn from_items(items: ItemDataStore) -> Diff {
        Diff {
            items,
            ..Default::default()
        }
    }

    // adjust the diff to the current state of the store
    // this is used when applying a diff to a store
    pub fn adjust(&self, store: &RefMut<DocStore>) -> Diff {
        let state = self.state.adjust_max(&store.state);

        // let next_state = &self.state + &state;

        let fields = self.fields.as_per(&store.fields);
        let mut items = ItemDataStore::default();

        // println!("before clients: {:?}", self.state.clients);
        // println!("after clients: {:?}", self.items);

        for (_, id_store) in self.items.iter() {
            for (_, item) in id_store.iter() {
                let adjust =
                    item.adjust(&self.state.clients, &self.fields, &state.clients, &fields);
                items.insert(adjust);
            }
        }

        let mut deletes = DeleteItemStore::default();

        for (_, store) in self.deletes.clone().into_iter() {
            for (_, item) in store.into_iter() {
                let adjust = item.adjust(&self.state.clients, &state.clients);
                deletes.insert(adjust);
            }
        }

        // drop the changes which are already integrated to the document
        let mut changes = self.changes.clone();
        self.changes.iter().for_each(|(id, change_store)| {
            change_store.iter().for_each(|change| {
                if store.changes.contains(&change.id()) {
                    changes.remove(&change.id());
                }
            })
        });

        Diff::from(
            self.doc_id.clone(),
            self.created_by.clone(),
            fields.clone(),
            self.changes.clone(),
            state.clone(),
            items,
            deletes,
        )
    }

    // adjust the diff to the current state of the store
    pub fn adjust_diff(&self, other: &Diff) -> Diff {
        // print_yaml(&self.state);
        // print_yaml(&other.state);

        // merge client states
        let state = self.state.adjust_max(&other.state);

        // print_yaml(&state);

        // merge fields
        let fields = self.fields.as_per(&other.fields);

        let mut items = ItemDataStore::default();

        // adjust items
        for (_, store) in self.items.iter() {
            for (_, item) in store.iter() {
                let adjust =
                    item.adjust(&self.state.clients, &self.fields, &state.clients, &fields);
                items.insert(adjust);
            }
        }

        // adjust deletes
        let mut deletes = DeleteItemStore::default();

        for (_, store) in self.deletes.clone().into_iter() {
            for (_, item) in store.into_iter() {
                let adjust = item.adjust(&self.state.clients, &state.clients);
                deletes.insert(adjust);
            }
        }

        Diff::from(
            self.doc_id.clone(),
            self.created_by.clone(),
            fields,
            self.changes.clone(),
            state,
            items,
            deletes,
        )
    }

    // merge two diffs together into self
    pub fn merge(&mut self, other: &Diff) {
        if self.doc_id != other.doc_id {
            panic!("cannot merge diffs with different doc ids");
        }
        if self.created_by != other.created_by {
            panic!("cannot merge diffs with different created_by");
        }

        self.fields = self.fields.merge(&other.fields);
        self.state = self.state.merge(&other.state);
        self.items = self.items.merge(&other.items);
        self.deletes = self.deletes.merge(&other.deletes);
    }

    /// optimize the diff for storage
    pub(crate) fn optimize(&mut self) {
        for (_, store) in self.items.items.iter_mut() {
            for (_, item) in store.iter_mut() {
                item.optimize();
            }
        }
    }
}

impl Serialize for Diff {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Diff", 7)?;
        s.serialize_field("doc_id", &self.doc_id)?;
        s.serialize_field("created_by", &self.created_by)?;
        s.serialize_field("fields", &self.fields)?;
        s.serialize_field("state", &self.state)?;
        s.serialize_field("changes", &self.changes)?;
        s.serialize_field("deletes", &self.deletes)?;
        s.serialize_field("items", &self.items)?;
        s.end()
    }
}

impl Encode for Diff {
    fn encode<E: Encoder>(&self, e: &mut E, cx: &mut EncodeContext) {
        self.doc_id.encode(e, cx);
        self.created_by.encode(e, cx);
        self.fields.encode(e, cx);
        self.state.encode(e, cx);
        self.deletes.encode(e, cx);
        self.items.encode(e, cx);
        self.changes.encode(e, cx);
    }
}

impl Decode for Diff {
    fn decode<D: Decoder>(d: &mut D, ctx: &DecodeContext) -> Result<Diff, String> {
        let doc_id = DocId::decode(d, ctx)?;
        let created_by = Client::decode(d, ctx)?;
        let fields = FieldMap::decode(d, ctx)?;
        let state = ClientState::decode(d, ctx)?;
        let deletes = DeleteItemStore::decode(d, ctx)?;
        let items = ItemDataStore::decode(d, ctx)?;
        let changes = ChangeStore::decode(d, ctx)?;

        Ok(Diff {
            doc_id,
            created_by,
            fields,
            changes,
            state,
            deletes,
            items,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::codec_v1::EncoderV1;
    use crate::decoder::Decode;
    use crate::diff::Diff;
    use crate::doc::Doc;
    use crate::encoder::{Encode, Encoder};
    use crate::state::ClientState;

    #[test]
    fn test_encode_decode_diff() {
        let doc = Doc::default();
        let text = doc.text();
        text.append(doc.string("hello"));

        doc.set("string", doc.string("str"));
        doc.set("text", text.clone());
        doc.set("props", doc.map());
        doc.set("k1", doc.atom("fe"));
        doc.set("k2", doc.list());

        let mut encoder = EncoderV1::default();

        let diff = doc.diff(ClientState::default());
        diff.encode(&mut encoder, &mut Default::default());

        let mut d = encoder.decoder();

        let decoded = Diff::decode(&mut d, &Default::default()).unwrap();

        assert_eq!(diff, decoded);
    }
}
