use std::collections::BTreeMap;

use crate::index::ItemIndexMap;
use crate::item::WithIndex;
use crate::Type;
use fractional_index::FractionalIndex;

/// The IBTree struct is a wrapper around a BTreeMap that allows for efficient
/// indexing of items based on their fractional index.
#[derive(Debug, Default)]
pub(crate) struct IBTree {
    pub(crate) btree: BTreeMap<FractionalIndex, Type>,
}

impl IBTree {
    pub(crate) fn new() -> Self {
        Self {
            btree: BTreeMap::new(),
        }
    }
}

impl ItemIndexMap<Type> for IBTree {
    fn size(&self) -> u32 {
        self.btree.len() as u32
    }

    fn at_index(&self, index: u32) -> Option<&Type> {
        self.btree.iter().nth(index as usize).map(|(_, v)| v)
    }

    fn index_of(&self, typ: &Type) -> i32 {
        self.btree.range(..typ.index()).count() as i32
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
