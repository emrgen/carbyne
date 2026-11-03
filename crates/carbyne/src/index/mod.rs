mod btee_index;
mod btree;
mod ibtree;
mod rbtree;
mod sbtree;
mod skiplist;
mod vecmap;

pub(crate) use btee_index::BTreeIndex;
pub(crate) use ibtree::IBTree;

use crate::Type;

pub(crate) trait ItemIndexMap<T> {
    fn size(&self) -> u32;
    fn at_index(&self, index: u32) -> Option<&T>;
    fn index_of(&self, item: &T) -> i32;
    fn insert(&mut self, item: T);
    fn remove(&mut self, item: &T);
    fn contains(&self, item: &T) -> bool;
}
