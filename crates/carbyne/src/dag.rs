use crate::bimapid::{ClientId, ClientMap, ClientMapper, FixedClientMapper};
use crate::change::{ChangeId, ChangeStore, ClientChangeId};
use crate::change_store::ClientStackStore;
use crate::decoder::{Decode, Decoder};
use crate::encoder::{Encode, Encoder};
use crate::frontier::Frontier;
use crate::id::{IdComp, WithId};
use crate::{Client, ClockTick, Id};
use bitflags::bitflags;
use rand::prelude::{SliceRandom, StdRng};
use rand::{Rng, SeedableRng};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use uuid::Uuid;

bitflags! {
    /// Flags for ChangeNode, currently unused but reserved for future use
    #[derive(Default)]
    pub(crate) struct ChangeNodeFlags: u8 {
        // flag to indicate if the change should be skipped
        const MOVE = 0b00000001;
        // flag to indicate if the change is a text change, consecutive text changes can be merged and undone incrementally
        const TEXT = 0b00000010;
    }
}

//     Default + WithId + Clone + Encode + Decode + Eq + PartialEq + Serialize
pub(crate) struct ChangeNode {
    flags: u8, // flags for future use, currently unused
    change: ChangeId,
    parents: Vec<ChangeId>,
}

impl ChangeNode {
    pub(crate) fn root(change: ChangeId) -> Self {
        Self {
            change,
            parents: Vec::new(),
            flags: 0, // no flags set
        }
    }

    #[inline]
    pub fn new(change: ChangeId, parents: Vec<ChangeId>) -> Self {
        Self {
            change,
            parents,
            flags: 0, // no flags set
        }
    }

    #[inline]
    pub(crate) fn skipped(mut self) -> Self {
        self
    }

    pub(crate) fn with_mover(mut self, moved: bool) -> Self {
        if moved {
            self.flags |= ChangeNodeFlags::MOVE.bits();
        }
        self
    }

    #[inline]
    pub(crate) fn client(&self) -> &ClientId {
        &self.change.client
    }
}

impl Default for ChangeNode {
    fn default() -> Self {
        Self {
            change: ChangeId::default(),
            parents: vec![],
            flags: 0, // no flags set
        }
    }
}

impl Debug for ChangeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl IdComp for ChangeNode {
    fn comp_id(&self, other: &Id) -> Ordering {
        self.change.comp_id(&other)
    }
}

impl Clone for ChangeNode {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl Ord for ChangeNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.change.cmp(&other.change)
    }
}

impl Eq for ChangeNode {}

impl PartialOrd for ChangeNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for ChangeNode {
    fn eq(&self, other: &Self) -> bool {
        self.change == other.change
    }
}

impl WithId for ChangeNode {
    fn id(&self) -> Id {
        self.change.id()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ChangeLinks {
    children: HashMap<Id, (u32, u32)>, // (total_count, current_count)
    dirty: HashSet<Id>,                // dirty children that need to be updated
}

impl ChangeLinks {
    fn add(&mut self, id: Id) {
        let entry = self.children.entry(id).or_insert((0, 0));
        entry.0 += 1; // Increment total count
        entry.1 += 1; // Increment current count
    }

    // when child is undone the parent current count is decremented by 1
    fn unlink_parent(&mut self, parent_id: Id) -> bool {
        if let Some(entry) = self.children.get_mut(&parent_id) {
            self.dirty.insert(parent_id);
            if entry.1 > 0 {
                entry.1 -= 1; // Decrement total count
            }
            entry.1 == 0
        } else {
            false // Parent not found, nothing to undo
        }
    }

    // check if the parent is ready to be undone
    fn is_ready(&self, parent_id: Id) -> bool {
        if let Some(entry) = self.children.get(&parent_id) {
            entry.1 == 0 // If current count is 0, it's ready to be undone
        } else {
            true // if no entry exists, consider it ready, in other words the parent has no children
        }
    }

    fn reset(&mut self) {
        self.dirty.iter().for_each(|id| {
            if let Some(entry) = self.children.get_mut(id) {
                entry.1 = entry.0;
            }
        });
        self.dirty.clear();
    }
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub(crate) struct ChangeDag {
    // store of changes, indexed by (client, clock)
    store: ClientStackStore<ChangeNode>,
    // links between changes and their children
    parents: ChangeLinks,
    // ready to be undone
    queue: BTreeSet<ClientChangeId>,
    // ends tracks the ends of the DAG for each client which is in the queue
    ends: HashMap<ClientId, ChangeId>,
    // dirty clients that need to be reset one the undo-do-redo is done
    dirty: HashSet<ClientId>,
    // clients map, used to map client ids to client objects
    pub(crate) clients: ClientMap,
}

impl ChangeDag {
    /// Creates a frontier for the DAG, which is the last change in the queue.
    // fn frontier(&self) -> Frontier {
    //     let change = self.queue.last().cloned().unwrap();
    //     Frontier::new(Id::new(change.client, change.end))
    // }

    // Insert a new change into the DAG.
    pub(crate) fn insert<T: ClientMapper>(
        &mut self,
        node: ChangeNode,
        client_map: &T,
    ) -> Result<(), String> {
        node.parents
            .iter()
            .for_each(|change_id| self.parents.add(change_id.id()));

        if let Some(last) = self.store.last(&node.client()) {
            if let Some(last) = last.change.to_client_change_id(client_map) {
                self.queue.remove(&last);
            }
        }

        if let Some(last) = node.change.to_client_change_id(client_map) {
            self.queue.insert(last);
        }

        // insert into ends
        self.ends.insert(node.change.client, node.change.clone());

        self.store.insert(node.change.client, node);

        Ok(())
    }

    // pop the last change from the store in topological order
    pub(crate) fn undo<T: ClientMapper>(&mut self, client_map: &T) -> Option<(ChangeId, u8)> {
        // pop the last change from the queue
        let last_id = self.queue.pop_last();
        if let Some(client_change_id) = last_id {
            if let Some(change_id) = client_change_id.to_change_id(client_map) {
                let cursor = self.store.cursor(change_id.client);

                // move the cursor to the previous change
                self.store.prev(change_id.client);
                self.dirty.insert(change_id.client);

                if let Some(cursor) = cursor {
                    self.store
                        .at_cursor(change_id.client, cursor)
                        .map(|node| &node.parents)
                        .map(|parents| {
                            parents.iter().for_each(|id| {
                                if self.parents.unlink_parent(id.id()) {
                                    if let Some(last) = self.store.current(id.client) {
                                        // if the last change has become ready to be undone,
                                        if last.id() == id.id() {
                                            if let Some(last) =
                                                last.change.to_client_change_id(client_map)
                                            {
                                                self.queue.insert(last);
                                            }
                                            self.ends
                                                .insert(last.change.client, last.change.clone());
                                        }
                                    }
                                }
                            });
                        });
                }

                if let Some(curr) = self.store.current(change_id.client) {
                    if self.parents.is_ready(curr.change.id()) {
                        if let Some(curr) = curr.change.to_client_change_id(client_map) {
                            self.queue.insert(curr);
                        }
                        self.ends.insert(curr.change.client, curr.change.clone());
                    }
                }

                let flags = self
                    .store
                    .find(change_id.id())
                    .map(|node| node.flags)
                    .unwrap_or_default();

                return Some((change_id, flags));
            }
        }

        None
    }

    // Reset the state of the DAG, clearing the queue and resetting the store
    pub(crate) fn done<T: ClientMapper>(&mut self, client_map: &T) {
        self.dirty.iter().for_each(|client_id| {
            self.store.reset_cursor(&client_id);
            if let Some(end) = self.ends.get(client_id) {
                if let Some(end) = end.to_client_change_id(client_map) {
                    self.queue.remove(&end);
                }
                if let Some(last) = self.store.last(client_id) {
                    if let Some(last) = last.change.to_client_change_id(client_map) {
                        self.queue.insert(last);
                    }
                }
            }
        });
    }

    // this is for testing purposes, to sort the changes in the order they were undone
    fn sort_changes<T: ClientMapper>(&mut self, client_map: &T) -> Vec<ChangeId> {
        let mut sorted_changes = Vec::new();

        while let Some((change_id, _)) = self.undo(client_map) {
            sorted_changes.push(change_id);
        }

        sorted_changes.reverse();

        sorted_changes
    }
}

// Testing utility to generate a random DAG
pub(crate) struct RandomDag {
    clients: Vec<ClientId>,
    ends: HashMap<ClientId, u32>,
    changes: Vec<ChangeId>,
    children: HashMap<ChangeId, Vec<ChangeId>>,
    parents: HashMap<ChangeId, Vec<ChangeId>>,
    rng: StdRng,
}

impl RandomDag {
    fn default() -> Self {
        Self::with_clients(1, 0)
    }

    pub(crate) fn children(&self) -> HashMap<(ClientId, ClockTick), Vec<(ClientId, ClockTick)>> {
        let children = self
            .children
            .iter()
            .map(|(k, v)| {
                let children = v.iter().map(|id| (id.client, id.start)).collect::<Vec<_>>();
                ((k.client, k.start), children)
            })
            .collect::<HashMap<_, _>>();

        children
    }

    pub(crate) fn parents(&self, child: &ChangeId) -> Vec<ChangeId> {
        self.parents.get(child).cloned().unwrap_or_default()
    }

    pub(crate) fn with_clients(count: u32, rand: u64) -> Self {
        let clients = (0..count).map(|i| i).collect::<Vec<ClientId>>();
        let change = ChangeId::new(0, 1, 1);
        let mut ends = HashMap::new();
        for client in &clients {
            ends.insert(client.clone(), 1);
        }
        // first client has already taken a clock of 1
        ends.insert(0, 2);

        Self {
            clients,
            ends,
            changes: vec![change],
            children: HashMap::new(),
            parents: HashMap::new(),
            rng: StdRng::seed_from_u64(rand),
        }
    }

    // randomly generate a DAG with the given number of nodes
    pub(crate) fn generate(&mut self, nodes: u32) {
        for _ in 0..nodes {
            let client = self.clients[self.rng.gen_range(0..self.clients.len())];
            // randomly choose parents
            let parent_count = self.rng.gen_range(1..=4);
            let mut parents = HashSet::new();
            for _ in 0..parent_count {
                if let Some(parent) = self.changes.get(self.rng.gen_range(0..self.changes.len())) {
                    parents.insert(parent.clone());
                }
            }

            let start = self.ends.get(&client).cloned().unwrap_or(1);
            let end = start + self.rng.gen_range(1..4);
            self.ends.insert(client.clone(), end + 1);

            // create a new change
            let change = ChangeId::new(client, start, end);
            self.changes.push(change.clone());

            // add the change to the links
            parents.iter().for_each(|parent| {
                self.children
                    .entry(parent.clone())
                    .or_insert_with(Vec::new)
                    .push(change.clone());
            });

            let parents = parents.iter().cloned().collect::<Vec<ChangeId>>();
            self.parents.insert(change.clone(), parents);
        }
    }

    // random topological sort of the changes
    pub(crate) fn sort(&mut self) -> Vec<ChangeId> {
        let mut done = HashSet::new();
        let mut sorted = Vec::new();
        let mut store = ChangeStore::default();
        let deps = self.parents.clone();
        for change_id in self.changes.iter() {
            store.insert(change_id.clone());
        }
        let mut size = store.size();
        let clients = self.clients.clone();
        while size > 0 {
            // choose random client
            if let Some(client) = clients.choose(&mut self.rng).cloned() {
                if let Some(store) = store.id_store_mut(&client) {
                    if let Some(first) = store.first().cloned() {
                        let ok = deps.get(&first).map_or(true, |parents| {
                            parents.iter().all(|parent| done.contains(parent))
                        });
                        if ok {
                            sorted.push(first.clone());
                            done.insert(first.clone());
                            store.pop_first();
                            size -= 1;
                        }
                    }
                }
            }
        }

        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::digest::HashMarker;
    use std::fmt::format;

    #[test]
    fn test_change_dag_insert_and_undo() {
        let mut dag = ChangeDag::default();
        let c1 = ChangeId::new(1, 0, 0);
        let c2 = ChangeId::new(1, 1, 1);
        let c3 = ChangeId::new(1, 2, 2);

        let mut client_map = FixedClientMapper::default();
        client_map.add(1, Client::UUID(Uuid::new_v4()));

        dag.insert(ChangeNode::root(c1), &client_map);
        dag.insert(ChangeNode::new(c2, vec![c1]), &client_map);
        dag.insert(ChangeNode::new(c3, vec![c2]), &client_map);

        assert_eq!(dag.queue.len(), 1);
        let item = dag.undo(&client_map);
        assert_eq!(item.unwrap().0, c3);

        assert_eq!(dag.queue.len(), 1);
        let item = dag.undo(&client_map);
        assert_eq!(item.unwrap().0, c2);

        assert_eq!(dag.queue.len(), 1);
        let item = dag.undo(&client_map);
        assert_eq!(item.unwrap().0, c1);

        dag.done(&client_map);

        let items = dag.sort_changes(&client_map);
        assert_eq!(items.len(), 3);
        assert_eq!(items, vec![c1, c2, c3]);
    }

    #[test]
    fn test_change_dag_insert_and_undo_1() {
        let mut dag = ChangeDag::default();
        let c1 = ChangeId::new(1, 1, 1);
        let c2 = ChangeId::new(1, 2, 3);
        let c3 = ChangeId::new(1, 4, 7);

        let mut client_map = FixedClientMapper::default();
        client_map.add(1, Client::UUID(Uuid::new_v4()));

        dag.insert(ChangeNode::root(c1), &client_map);
        dag.insert(ChangeNode::new(c2, vec![c1]), &client_map);
        dag.insert(ChangeNode::new(c3, vec![c1]), &client_map);

        let items = dag.sort_changes(&client_map);
        assert_eq!(items.len(), 3);
        assert_eq!(items, vec![c1, c2, c3]);
    }

    #[test]
    fn test_change_dag_insert_and_undo_2() {
        for i in 0..1000 {
            let mut dag = ChangeDag::default();
            let c1 = ChangeId::new(0, 1, 1);
            let c2 = ChangeId::new(0, 2, 5);
            let c3 = ChangeId::new(0, 6, 9);
            let c4 = ChangeId::new(1, 1, 2);
            let c5 = ChangeId::new(1, 3, 5);

            let mut client_map = FixedClientMapper::default();
            client_map.add(0, Client::UUID(Uuid::new_v4()));
            client_map.add(1, Client::UUID(Uuid::new_v4()));

            dag.insert(ChangeNode::root(c1), &client_map);
            dag.insert(ChangeNode::new(c2, vec![c1]), &client_map);
            dag.insert(ChangeNode::new(c3, vec![c1]), &client_map);
            dag.insert(ChangeNode::new(c4, vec![c3]), &client_map);
            dag.insert(ChangeNode::new(c5, vec![c3, c4]), &client_map);

            let items = dag.sort_changes(&client_map);
            assert_eq!(items.len(), 5);
            // assert_eq!(items, vec![c1, c2, c3, c4, c5]);
        }
    }

    #[test]
    fn generate_random_dag() {
        let mut rng = rand::thread_rng();
        let mut dag = RandomDag::with_clients(10, 2);
        let mut client_map = FixedClientMapper::default();
        for i in 0..10 {
            client_map.add(i, Client::UUID(Uuid::new_v4()));
        }

        dag.generate(1000);

        // println!("{:?}", dag.children);
        // println!("{:?}", dag.sort());

        let mut ch_dag1 = ChangeDag::default();
        let sort1 = dag.sort();
        for change in &sort1 {
            let parents = dag.parents.get(&change).cloned().unwrap_or_default();
            ch_dag1.insert(ChangeNode::new(change.clone(), parents), &client_map);
        }
        let sorted_changes1 = ch_dag1.sort_changes(&client_map);

        // fuzz test, for different topological sort must converge
        for i in 0..500 {
            let mut ch_dag2 = ChangeDag::default();
            let sort2 = dag.sort();
            for change in &sort2 {
                let parents = dag.parents.get(&change).cloned().unwrap_or_default();
                ch_dag2.insert(ChangeNode::new(change.clone(), parents), &client_map);
            }
            let sorted_changes2 = ch_dag2.sort_changes(&client_map);

            assert_ne!(sort1, sort2);
            assert_eq!(sorted_changes1, sorted_changes2)
        }
    }
}
