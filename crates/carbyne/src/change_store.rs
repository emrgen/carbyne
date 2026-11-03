use crate::bimapid::ClientId;
use crate::decoder::Decode;
use crate::encoder::Encode;
use crate::id::{IdComp, WithId, WithIdRange};
use crate::store::ItemStoreEntry;
use crate::Id;
use hashbrown::{HashMap, HashSet};
use serde::Serialize;
use std::collections::hash_map::Iter;

pub(crate) trait ItemStackStoreEntry:
    WithId + IdComp + Clone + Eq + PartialEq + Ord
{
}

impl<T: WithId + IdComp + Clone + Eq + PartialEq + Ord> ItemStackStoreEntry for T {}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct ClientStackStore<T: ItemStackStoreEntry> {
    map: HashMap<ClientId, ItemStackStore<T>>,
}

impl<T: ItemStackStoreEntry + Default> ClientStackStore<T> {
    pub(crate) fn insert(&mut self, client_id: ClientId, item: T) {
        let entry = self
            .map
            .entry(client_id)
            .or_insert_with(|| ItemStackStore::default());
        entry.push(item);
    }

    pub(crate) fn iter(&self) -> hashbrown::hash_map::Iter<'_, ClientId, ItemStackStore<T>> {
        self.map.iter()
    }

    pub(crate) fn reset_cursor(&mut self, client_id: &ClientId) {
        if let Some(entry) = self.map.get_mut(client_id) {
            entry.reset_cursor();
        }
    }

    pub(crate) fn find(&self, id: Id) -> Option<&T> {
        self.map.get(&id.client).and_then(|store| store.find(id))
    }

    pub(crate) fn prev(&mut self, client_id: ClientId) -> Option<&T> {
        self.map.get_mut(&client_id).and_then(|store| store.prev())
    }

    pub(crate) fn next(&mut self, client_id: ClientId) -> Option<&T> {
        self.map.get_mut(&client_id).and_then(|store| store.next())
    }

    pub(crate) fn cursor(&self, client_id: ClientId) -> Option<usize> {
        self.map.get(&client_id).map(|store| store.cursor)
    }

    pub(crate) fn at_cursor(&self, client_id: ClientId, cursor: usize) -> Option<&T> {
        self.map.get(&client_id).and_then(|store| {
            if cursor <= store.items.len() {
                store.items.get(cursor - 1)
            } else {
                None
            }
        })
    }

    pub(crate) fn current(&self, client_id: ClientId) -> Option<&T> {
        self.map.get(&client_id).and_then(|store| store.current())
    }

    pub(crate) fn pop(&mut self, client_id: ClientId) -> Option<T> {
        self.map.get_mut(&client_id).and_then(|store| store.pop())
    }

    pub(crate) fn last(&self, client_id: &ClientId) -> Option<&T> {
        self.map.get(client_id).and_then(|store| store.last())
    }

    pub(crate) fn reset(&mut self, client_id: ClientId) {
        if let Some(store) = self.map.get_mut(&client_id) {
            store.reset_cursor();
        }
    }

    pub(crate) fn last_mut(&mut self, client_id: ClientId) -> Option<&mut T> {
        self.map
            .get_mut(&client_id)
            .and_then(|store| store.last_mut())
    }

    pub(crate) fn changes(&self, client_id: ClientId) -> Option<&[T]> {
        self.map.get(&client_id).map(|store| store.changes())
    }

    pub(crate) fn clear(&mut self, client_id: ClientId) {
        if let Some(entry) = self.map.get_mut(&client_id) {
            entry.clear();
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct ItemStackStore<T: ItemStackStoreEntry> {
    items: Vec<T>,
    cursor: usize,
}

impl<T: ItemStackStoreEntry> ItemStackStore<T> {
    pub(crate) fn push(&mut self, item: T) {
        if self
            .items
            .last()
            .map(|last| last.id() == item.id())
            .unwrap_or(false)
        {
            return; // No need to insert if the item is already the last one
        } else {
            self.items.push(item);
            self.cursor += 1;
        }
    }

    // find using binary search
    pub(crate) fn find(&self, id: Id) -> Option<&T> {
        // NOTE: assuming items are from same client and sorted by id
        // will not work if items are not from same client
        let index = self.items.binary_search_by(|item| item.comp_id(&id));
        if let Ok(idx) = index {
            Some(&self.items[idx])
        } else {
            None
        }
    }

    pub(crate) fn reset_cursor(&mut self) -> usize {
        self.cursor = self.items.len();
        self.cursor
    }

    pub(crate) fn prev(&mut self) -> Option<&T> {
        if self.cursor > 0 {
            self.cursor -= 1;
        }

        if self.cursor > 1 {
            self.items.get(self.cursor - 1)
        } else {
            None
        }
    }

    pub(crate) fn next(&mut self) -> Option<&T> {
        if self.cursor < self.items.len() {
            self.cursor += 1;
        }

        if self.cursor <= self.items.len() {
            self.items.get(self.cursor - 1)
        } else {
            None
        }
    }

    pub(crate) fn current(&self) -> Option<&T> {
        if self.cursor == 0 {
            return None;
        }

        self.items.get(self.cursor - 1)
    }

    pub(crate) fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    pub(crate) fn last(&self) -> Option<&T> {
        self.items.last()
    }

    pub(crate) fn last_mut(&mut self) -> Option<&mut T> {
        self.items.last_mut()
    }

    pub(crate) fn changes(&self) -> &[T] {
        &self.items
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::{Change, ChangeId};
    use crate::id::Id;

    #[test]
    fn test_find_change_id() {
        let mut dag = ItemStackStore::default();
        let c1 = ChangeId::new(1, 0, 0);
        dag.push(c1.clone());
        let c2 = ChangeId::new(1, 2, 5);
        dag.push(c2.clone());
        let c3 = ChangeId::new(1, 6, 8);
        dag.push(c3.clone());
        let c4 = ChangeId::new(1, 9, 10);
        dag.push(c4.clone());
        let c5 = ChangeId::new(1, 11, 14);
        dag.push(c5.clone());
        let c6 = ChangeId::new(1, 15, 20);
        dag.push(c6.clone());

        assert_eq!(dag.find(c1.id()), Some(&c1));
        assert_eq!(dag.find(c2.id()), Some(&c2));
        assert_eq!(dag.find(c3.id()), Some(&c3));
        assert_eq!(dag.find(c4.id()), Some(&c4));
        assert_eq!(dag.find(c5.id()), Some(&c5));
        assert_eq!(dag.find(c6.id()), Some(&c6));
        assert_eq!(dag.find(Id::new(1, 7)), Some(&c3));
    }
}
