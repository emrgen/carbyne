use crate::index::ItemIndexMap;
use crate::item::WithIndex;
use crate::Type;
use btree_slab::BTreeMap;
use fractional_index::FractionalIndex;
use std::fmt::Debug;

pub(crate) struct SlabBTree {
    btree: BTreeMap<FractionalIndex, Type>,
}

impl Debug for SlabBTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ABTree").finish()
    }
}

impl Default for SlabBTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SlabBTree {
    pub(crate) fn new() -> Self {
        Self {
            btree: BTreeMap::new(),
        }
    }
}

impl ItemIndexMap<Type> for SlabBTree {
    fn size(&self) -> u32 {
        self.btree.len() as u32
    }

    fn at_index(&self, index: u32) -> Option<&Type> {
        self.btree.iter().nth(index as usize).map(|(_, v)| v)
    }

    fn index_of(&self, index: &Type) -> i32 {
        self.btree.range(..index.index()).count() as i32
    }

    fn insert(&mut self, value: Type) {
        self.btree.insert(value.index(), value);
    }

    fn remove(&mut self, item: &Type) {
        self.btree.remove(&item.index());
    }

    fn contains(&self, item: &Type) -> bool {
        self.btree.contains_key(&item.index())
    }
}
