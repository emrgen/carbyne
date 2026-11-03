use crate::index::btree::BTree;
use crate::index::ItemIndexMap;
use crate::item::WithIndex;
use crate::Type;
use fractional_index::FractionalIndex;
use std::fmt::Display;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Index(FractionalIndex);

impl Display for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

#[derive(Debug, Default)]
pub(crate) struct BTreeIndex {
    tree: BTree<Index, Type>,
}

impl ItemIndexMap<Type> for BTreeIndex {
    fn size(&self) -> u32 {
        self.tree.size() as u32
    }

    fn at_index(&self, index: u32) -> Option<&Type> {
        self.tree.at_index(index as usize)
    }

    fn index_of(&self, item: &Type) -> i32 {
        self.tree
            .index_of(&Index(item.index()))
            .map_or(-1, |i| i as i32)
    }

    fn insert(&mut self, item: Type) {
        self.tree.insert(Index(item.index()), item);
    }

    fn remove(&mut self, item: &Type) {
        todo!()
    }

    fn contains(&self, item: &Type) -> bool {
        todo!()
    }
}
