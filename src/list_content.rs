use crate::index_tree::{IndexTree, ValueIter};
use std::fmt::Debug;
use std::hash::Hash;

// expose a vec like api for get, insert, remove, index_of
#[derive(Debug)]
pub(crate) enum ListContent<T: Debug + Clone + Eq + Hash> {
    Small(Vec<T>),
    Large(IndexTree<T>),
}

impl<T: Debug + Clone + Eq + Hash> Default for ListContent<T> {
    fn default() -> Self {
        ListContent::Small(Vec::new())
    }
}

impl<T: Debug + Clone + Eq + Hash> ListContent<T> {
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        match self {
            ListContent::Small(v) => v.get(index),
            ListContent::Large(v) => v.at_index(index),
        }
    }

    fn is_small(&self) -> bool {
        match self {
            ListContent::Small(_) => true,
            ListContent::Large(_) => false,
        }
    }

    // auto upgrade to index tree when item count reach above a threshold
    pub(crate) fn insert(&mut self, index: usize, item: T) {
        // the small container size limit is a runtime config
        if self.is_small() && self.size() == 16 {
            let mut tree = IndexTree::new();
            match self {
                ListContent::Small(v) => v.drain(0..).enumerate().for_each(|(i, t)| {
                    tree.insert(i, t);
                }),
                _ => {}
            };

            *self = ListContent::Large(tree)
        }

        match self {
            ListContent::Small(v) => {
                v.insert(index, item);
            }
            ListContent::Large(v) => {
                v.insert(index, item);
            }
        }
    }

    // auto downgrade to vec when item count reach below a threshold
    pub(crate) fn remove(&self, index: usize) -> Option<&T> {
        None
    }

    pub(crate) fn first(&self) -> Option<&T> {
        self.get(0)
    }

    pub(crate) fn last(&self) -> Option<&T> {
        self.get(self.size() - 1)
    }

    pub(crate) fn append(&mut self, item: T) {
        self.insert(self.size(), item);
    }

    pub(crate) fn prepend(&mut self, item: T) {
        self.insert(0, item);
    }

    pub(crate) fn remove_item(&mut self, item: &T) {
        if let Some(index) = self.index_of(item) {
            self.remove(index);
        }
    }

    pub(crate) fn index_of(&self, item: &T) -> Option<usize> {
        match self {
            ListContent::Small(v) => v.iter().position(|r| r == item),
            ListContent::Large(v) => v.index_of(item),
        }
    }

    pub(crate) fn size(&self) -> usize {
        match self {
            ListContent::Small(v) => v.len(),
            ListContent::Large(v) => v.size(),
        }
    }

    fn contains(&self, item: &T) -> bool {
        match self {
            ListContent::Small(v) => v.iter().position(|r| r == item).map_or(false, |v| true),
            ListContent::Large(v) => v.contains(item),
        }
    }

    // iterate over all list items
    pub(crate) fn iter(&self) -> ListContentIter<'_, T> {
        match self {
            ListContent::Small(v) => ListContentIter::Small(v.iter()),
            ListContent::Large(v) => ListContentIter::Large(v.iter_from(0).unwrap()),
        }
    }
}

enum ListContentIter<'a, T: Debug> {
    Small(std::slice::Iter<'a, T>),
    Large(ValueIter<'a, T>),
}

impl<'a, V: Debug> Iterator for ListContentIter<'a, V> {
    type Item = (&'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ListContentIter::Small(v) => v.next(),
            ListContentIter::Large(v) => v.next(),
        }
    }
}
