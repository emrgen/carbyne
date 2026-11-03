use crate::bimapid::{ClientId, ClientMap, ClientMapper};
use crate::change::ChangeId;
use crate::change_btree::BTree;
use crate::hash::calculate_hash;
use crate::id::WithId;
use crate::{ClockTick, Id};
use btree_slab::{BTreeMap, BTreeSet};
use fractional_index::FractionalIndex;
use hashbrown::{HashMap, HashSet};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

#[derive(Clone)]
struct ChildHashId {
    id: Id,
    hash: u64,
}

impl Eq for ChildHashId {}

impl PartialEq<Self> for ChildHashId {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd<Self> for ChildHashId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChildHashId {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.id.client.eq(&other.id.client) {
            return self.id.clock.cmp(&other.id.clock);
        }

        self.hash.cmp(&other.hash)
    }
}

struct ChangeListNode {
    flag: u8,
    index: FractionalIndex,
    children: Vec<ChildHashId>,
}

struct ChangeList {
    changes: HashMap<Id, ChangeListNode>,
    index_map: BTree<FractionalIndex, ChangeId>, // sorted changes
    moves: BTree<FractionalIndex, ChangeId>,     // sorted mover changes
}

impl ChangeList {
    pub fn new() -> Self {
        ChangeList {
            index_map: BTree::new(10),
            changes: HashMap::new(),
            moves: BTree::new(10),
        }
    }

    pub(crate) fn insert_root(&mut self, change_id: &ChangeId) {
        let node = ChangeListNode {
            index: FractionalIndex::default(),
            children: Vec::new(),
            flag: 0,
        };

        self.index_map.insert(node.index.clone(), change_id.clone());
        self.changes.insert(change_id.id(), node);
    }

    // insert the
    pub(crate) fn insert<T: ClientMapper>(
        &mut self,
        change_id: &ChangeId,
        parent_id: &ChangeId,
        flags: u8,
        client_map: &T,
    ) -> usize {
        if !self.changes.contains_key(&parent_id.id()) {
            panic!("Parent change ID does not exist in the change list.");
        }

        let id = change_id.id();
        let child_hash_id = ChildHashId {
            id: id.clone(),
            hash: calculate_hash(&client_map.get_client(&id.client)),
        };

        let (parent_node, pos): (&ChangeListNode, usize) = {
            let parent_node = self.changes.get_mut(&parent_id.id()).unwrap();

            let pos = parent_node
                .children
                .partition_point(|k| k <= &child_hash_id);

            // println!("insert pos: {}", pos);

            // Insert the new change ID into the parent's children vector
            parent_node.children.insert(pos, child_hash_id.clone());

            (parent_node, pos)
        };

        // find the prev node of the current change in preorder traversal of the change tree
        let prev_frac_index = if pos == 0 {
            parent_node.index.clone()
        } else {
            let sibling = parent_node.children.get(pos - 1).cloned().unwrap();
            let mut last_node = self.changes.get(&sibling.id).unwrap();
            let mut last_id = sibling.id.clone();
            while let Some(last) = last_node.children.last() {
                last_node = self.changes.get(&last.id).unwrap();
                last_id = last.id.clone();
            }

            // println!("insert prev item: {:?}", last_id);
            last_node.index.clone()
        };

        // println!("insert prev item: {:?}", prev_item.id());

        let items = self
            .index_map
            .iter()
            .map(|cid| return (cid.0, cid.1.tuple()))
            .collect::<Vec<_>>();

        // println!("items: {:?}", items);

        let prev_index = self.index_map.index_of(&prev_frac_index).unwrap();
        // println!(
        //     "insert prev item index: {:?}, size: {}",
        //     prev_index,
        //     self.index_map.size()
        // );
        // println!(
        //     "insert items next item: {:?}",
        //     self.index_map.at_index(prev_index + 1)
        // );

        let next_frac_index = self
            .index_map
            .at_index(prev_index + 1)
            .map(|n| {
                // println!("insert next item: {:?}", n);
                self.changes.get(&n.id())
            })
            .map(|n| n.map(|n| n.index.clone()))
            .flatten();

        // create frac index for the change_id
        let frac_index = match (&next_frac_index) {
            Some(next_frac_index) => {
                FractionalIndex::new_between(&prev_frac_index, &next_frac_index)
            }
            _ => Option::from({ FractionalIndex::new_after(&prev_frac_index) }),
        }
        .unwrap_or_else(|| {
            panic!(
                "failed to create a change frac index: prev: {:?}, next: {:?}",
                prev_frac_index, next_frac_index
            );
        });

        // println!(
        //     "prev: {:?}, next: {:?}, frac: {:?}",
        //     prev_frac_index, next_frac_index, frac_index
        // );

        // Insert the change into the index map and changes map
        let node = ChangeListNode {
            flag: flags,
            index: frac_index.clone(),
            children: Vec::new(),
        };

        // If the flags indicate a move, insert it into the moves map
        if flags > 0 {
            self.moves.insert(frac_index, change_id.clone());
        }
        self.index_map.insert(node.index.clone(), change_id.clone());
        self.changes.insert(id, node);

        pos
    }

    pub(crate) fn index_of(&self, change_id: &ChangeId) -> Option<usize> {
        self.changes
            .get(&change_id.id())
            .map(|node| self.index_map.index_of(&node.index))
            .flatten()
    }

    pub(crate) fn contains(&self, change_id: &ChangeId) -> bool {
        self.changes.contains_key(&change_id.id())
    }

    pub(crate) fn sort_changes(&self) -> Vec<(ClientId, ClockTick)> {
        self.index_map.iter().map(|(_, cid)| cid.tuple()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bimapid::FixedClientMapper;
    use crate::dag::{ChangeDag, ChangeNode, RandomDag};
    use crate::Client;
    use uuid::Uuid;

    #[test]
    fn test_insert_changes() {
        let mut list = ChangeList::new();
        let c1 = ChangeId::new(1, 1, 1);
        let c2 = ChangeId::new(1, 2, 2);
        let c3 = ChangeId::new(1, 3, 3);
        let c4 = ChangeId::new(1, 4, 4);
        let c5 = ChangeId::new(1, 5, 5);
        let mut clients = FixedClientMapper::default();
        clients.add(1, Client::UUID(Uuid::new_v4()));

        list.insert_root(&c1);

        list.insert(&c3, &c1, 0, &clients);
        list.insert(&c2, &c1, 0, &clients);
        list.insert(&c4, &c3, 0, &clients);
        list.insert(&c5, &c3, 0, &clients);

        // println!("index c1: {:?}", list.index_of(&c1));
        // println!("index c2: {:?}", list.index_of(&c2));
        // println!("index c3: {:?}", list.index_of(&c3));
        // println!("index c4: {:?}", list.index_of(&c4));
        // println!("index c5: {:?}", list.index_of(&c5));

        assert_eq!(list.changes.len(), 5);
        assert_eq!(list.index_of(&c1), Some(0));
        assert_eq!(list.index_of(&c2), Some(1));
        assert_eq!(list.index_of(&c3), Some(2));
        assert_eq!(list.index_of(&c4), Some(3));
        assert_eq!(list.index_of(&c5), Some(4));
    }

    fn insert_into<T: ClientMapper>(
        list: &mut ChangeList,
        dag: &RandomDag,
        clients: &T,
        child: &ChangeId,
    ) {
        let parents = dag.parents(child);
        let parent = parents.iter().max_by_key(|k| list.index_of(k));
        if let Some(parent) = parent {
            let pos = list.insert(child, parent, 0, clients);
            // println!(
            //     "{:?} -> {:?}, {} | index: {:?}",
            //     parent.tuple(),
            //     child.tuple(),
            //     pos,
            //     list.index_of(parent)
            // );
        } else {
            list.insert_root(child);
        }
        // println!("dag: {:?} \n----------", list.sort_changes());
    }

    #[test]
    fn test_change_list_generate_random_dag() {
        let clients = vec![
            Client::UUID(Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d1").unwrap()),
            Client::UUID(Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d2").unwrap()),
            Client::UUID(Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d3").unwrap()),
        ];

        let client_count = 2;

        for ii in 0..50 {
            let mut dag = RandomDag::with_clients(client_count, ii);
            let mut client_map = FixedClientMapper::default();
            for i in 0..client_count {
                client_map.add(i, clients.get(i as usize).cloned().unwrap());
            }
            dag.generate(100);

            // println!("{:?}", dag.children());

            let sort1 = dag.sort();
            let mut list = ChangeList::new();
            sort1.iter().for_each(|id| {
                // println!("count: {}, id: {:?}", count, id);
                insert_into(&mut list, &dag, &client_map, id);
            });
            let sorted_changes1 = list.sort_changes();

            // println!("sorted dag: {:?}", sort1);
            // println!("changes 1: {:?}", list.sort_changes());

            // println!("-----------------------------");

            for i in 0..50 {
                let sort2 = dag.sort();
                let mut list = ChangeList::new();
                sort2.iter().for_each(|id| {
                    insert_into(&mut list, &dag, &client_map, id);
                });

                let sorted_changes2 = list.sort_changes();
                // println!("sorted dag: {:?}", sort2);
                // println!("changes 2: {:?}", list.sort_changes());

                // the change integration may happen in any order
                assert_ne!(sort1, sort2);
                assert_eq!(sorted_changes1, sorted_changes2);
            }
        }
    }
}
