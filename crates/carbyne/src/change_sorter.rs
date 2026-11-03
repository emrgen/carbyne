// keep the changes in sorted order for the document

use crate::index_map::{IndexMap, IndexMapper, IndexRef};
use crate::tx::TxOp::Insert;
use crate::Id;
use ptree::{print_tree, TreeBuilder};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::io::Write;
use std::mem;

// ChangeTree is a BTree like append-only structure
// It has a simplified vector like API for efficient insertion by index.
// Support index look up and iter api
struct ChangeTree<V: Clone + Hash + Eq + Debug> {
    root: Node<V>,
    order: usize,
    dirty: HashSet<V>,
    refs: HashMap<V, IndexRef>,
    mapper: IndexMapper,
}

impl<V: Clone + Hash + Eq + Debug> ChangeTree<V> {
    pub(crate) fn new() -> Self {
        Self::with_order(10)
    }

    fn with_order(k: usize) -> Self {
        ChangeTree {
            root: Node::leaf(k),
            order: k,
            dirty: Default::default(),
            refs: Default::default(),
            mapper: Default::default(),
        }
    }

    // compress the index mappers
    fn compress(&mut self) {
        for id in &self.dirty {
            // update the index_ref with current index
            if let Some(idx_ref) = self.refs.get_mut(&id) {
                idx_ref.index = self.mapper.map_ref(idx_ref);
                idx_ref.mapper = 0;
            }
        }

        self.dirty.clear()
    }

    pub(crate) fn insert(&mut self, index: usize, value: V) {
        self.refs.insert(
            value.clone(),
            IndexRef::new(index, self.mapper.len() as u32),
        );
        self.mapper.push(IndexMap::insert(index));

        if let Some(node) = self.root.insert(index, value) {
            let mut new_root = Node::Branch(Branch::new(self.order));
            let old_root = mem::replace(&mut self.root, new_root);
            match self.root {
                Node::Branch(ref mut branch) => {
                    branch.children.push(old_root);
                    branch.children.push(node);
                    branch.update_count(0);
                }
                _ => unreachable!(),
            }
        }
    }

    pub(crate) fn index_of(&self, value: &V) -> Option<usize> {
        if let Some(index_ref) = self.refs.get(value) {
            Some(self.mapper.map_ref(index_ref))
        } else {
            None
        }
    }

    pub(crate) fn at_index(&self, index: usize) -> Option<&V> {
        if index >= self.root.size() {
            return None;
        }
        self.root.at_index(index)
    }

    pub(crate) fn at_index_mut(&mut self, index: usize) -> Option<&mut V> {
        if index >= self.root.size() {
            return None;
        }
        self.root.at_index_mut(index)
    }

    pub(crate) fn contains(&self, value: &V) -> bool {
        self.refs.contains_key(value)
    }

    pub(crate) fn size(&self) -> usize {
        self.root.size()
    }

    pub(crate) fn print(&self) {
        let mut tree = TreeBuilder::new(format!("BTree: {:?}", self.size()));
        self.root.print(&mut tree);
        print_tree(&tree.build()).unwrap();
    }

    fn iter_from(&self, index: usize) -> Option<ValueIter<V>> {
        let mut internals = Vec::new();
        self.root.iter_from(index, internals)
    }
}

enum Node<V> {
    Branch(Branch<V>),
    Leaf(Leaf<V>),
}

impl<V: std::fmt::Debug> Node<V> {
    fn branch(order: usize) -> Self {
        Node::Branch(Branch::new(order))
    }

    fn leaf(order: usize) -> Self {
        Node::Leaf(Leaf::new(order))
    }

    pub(crate) fn insert(&mut self, index: usize, value: V) -> Option<Node<V>> {
        match self {
            Node::Branch(branch) => branch.insert(index, value),
            Node::Leaf(leaf) => leaf.insert(index, value),
        }
    }

    pub(crate) fn at_index(&self, index: usize) -> Option<&V> {
        match self {
            Node::Branch(branch) => branch.at_index(index),
            Node::Leaf(leaf) => leaf.at_index(index),
        }
    }

    pub(crate) fn at_index_mut(&mut self, index: usize) -> Option<&mut V> {
        match self {
            Node::Branch(branch) => branch.at_index_mut(index),
            Node::Leaf(leaf) => leaf.at_index_mut(index),
        }
    }

    fn size(&self) -> usize {
        match self {
            Node::Branch(branch) => branch.size(),
            Node::Leaf(leaf) => leaf.size(),
        }
    }

    pub(crate) fn print(&self, tree: &mut TreeBuilder) {
        match self {
            Node::Leaf(leaf) => leaf.print(tree),
            Node::Branch(branch) => branch.print(tree),
        }
    }

    fn iter_from<'a>(
        &'a self,
        index: usize,
        mut internals: Vec<(&'a Node<V>, usize)>,
    ) -> Option<ValueIter<'a, V>> {
        match self {
            Node::Branch(branch) => {
                if let Some((pos, index)) = branch.get_child_with_index(index) {
                    if let Some(child) = branch.children.get(pos) {
                        internals.push((self, pos));
                        return child.iter_from(index, internals);
                    }
                }

                None
            }
            Node::Leaf(leaf) => Some(ValueIter {
                internals,
                leaf: self,
                index,
            }),
        }
    }
}

struct Branch<V> {
    children: Vec<Node<V>>,
    counts: Vec<usize>,
}

impl<V: Debug> Branch<V> {
    fn new(order: usize) -> Self {
        Branch {
            children: Vec::with_capacity(order),
            counts: vec![0, order],
        }
    }

    pub(crate) fn insert(&mut self, index: usize, value: V) -> Option<Node<V>> {
        if let Some((pos, index)) = self.get_child_with_index(index) {
            if let Some(child) = self.children.get_mut(pos) {
                if let Some(new_node) = child.insert(index, value) {
                    if self.is_full() {
                        return Some(self.insert_and_split(pos, new_node));
                    } else {
                        self.children.insert(pos + 1, new_node);
                        self.update_count(0);
                    }
                } else {
                    self.update_count(0);
                }
            }
        }

        None
    }

    fn insert_and_split(&mut self, pos: usize, new_child: Node<V>) -> Node<V> {
        let order = self.children.capacity();
        let mid = order / 2;

        let mut new_branch = Branch::new(self.children.capacity());

        self.children.insert(pos + 1, new_child);

        // TODO: potential BUG might be here
        new_branch.children.extend(self.children.drain(mid..));

        self.children.truncate(mid);

        new_branch.children.shrink_to(order);
        self.children.shrink_to(order);

        self.update_count(0);
        new_branch.update_count(0);

        Node::Branch(new_branch)
    }

    pub(crate) fn at_index(&self, index: usize) -> Option<&V> {
        if let Some((pos, index)) = self.get_child_with_index(index) {
            self.children
                .get(pos)
                .map(|node| node.at_index(index))
                .flatten()
        } else {
            None
        }
    }

    pub(crate) fn at_index_mut(&mut self, index: usize) -> Option<&mut V> {
        if let Some((pos, index)) = self.get_child_with_index(index) {
            self.children
                .get_mut(pos)
                .map(|node| node.at_index_mut(index))
                .flatten()
        } else {
            None
        }
    }

    #[inline]
    fn get_child_with_index(&self, index: usize) -> Option<(usize, usize)> {
        let pos = self
            .counts
            .partition_point(|&n| n <= index)
            .min(self.children.len() - 1);

        let index = if pos == 0 {
            index
        } else {
            index - self.counts[pos - 1]
        };

        Some((pos, index))
    }

    // TODO: this function is taking a lot of cycles during each insert
    #[inline]
    fn update_count(&mut self, start: usize) {
        let mut count = if start == 0 {
            0
        } else {
            self.counts[start - 1]
        };
        for (index, child) in self.children[start..].iter().enumerate() {
            count += child.size();
            self.counts.insert(start + index, count);
        }

        self.counts.truncate(self.children.len());
    }

    fn is_full(&self) -> bool {
        self.children.len() == self.children.capacity()
    }

    fn size(&self) -> usize {
        self.counts.last().copied().unwrap_or(0)
    }

    fn print(&self, tree: &mut TreeBuilder) {
        tree.begin_child(format!(
            "Branch: {}, counts: {:?}",
            self.children.len(),
            self.counts
        ));
        for (i, child) in self.children.iter().enumerate() {
            // tree.begin_child(format!("Count: {:?}", self.counts[i]));
            child.print(tree);
            // tree.end_child();
        }

        tree.end_child();
    }

    fn iter_from<'a>(
        &'a self,
        index: usize,
        mut internals: Vec<(&'a Node<V>, usize)>,
    ) -> Option<ValueIter<'a, V>> {
        if let Some((pos, index)) = self.get_child_with_index(index) {
            if let Some(child) = self.children.get(pos) {
                internals.push((child, pos));
                return child.iter_from(index, internals);
            }
        }

        None
    }
}

struct Leaf<V> {
    values: Vec<V>,
}

impl<V: std::fmt::Debug> Leaf<V> {
    fn new(order: usize) -> Self {
        Leaf {
            values: Vec::with_capacity(order),
        }
    }

    // Insert the value at index, splitting the node if full
    pub(crate) fn insert(&mut self, index: usize, value: V) -> Option<Node<V>> {
        if self.values.len() == self.values.capacity() {
            let mid = self.values.len() / 2;
            let mut new_leaf = Leaf::new(self.values.capacity());
            new_leaf.values.extend(self.values.drain(mid..));
            if index < mid {
                self.values.insert(index, value);
            } else {
                new_leaf.values.insert(index - mid, value);
            }
            Some(Node::Leaf(new_leaf))
        } else {
            self.values.insert(index, value);
            None
        }
    }

    pub(crate) fn at_index(&self, index: usize) -> Option<&V> {
        self.values.get(index)
    }

    pub(crate) fn at_index_mut(&mut self, index: usize) -> Option<&mut V> {
        self.values.get_mut(index)
    }

    fn size(&self) -> usize {
        self.values.len()
    }

    fn print(&self, tree: &mut TreeBuilder) {
        let mut string_builder = String::new();
        string_builder.push_str(&format!("Leaf: {} => ", self.values.capacity()));
        for (value) in self.values.iter() {
            string_builder.push_str(&format!("{:?}, ", value));
        }
        tree.begin_child(string_builder).end_child();
    }
}

pub(crate) struct ValueIter<'a, V: std::fmt::Debug> {
    internals: Vec<(&'a Node<V>, usize)>,
    leaf: &'a Node<V>,
    index: usize,
}

impl<'a, V: Debug> Iterator for ValueIter<'a, V> {
    type Item = (&'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.leaf.size() {
            let value = self.leaf.at_index(self.index)?;
            self.index += 1;

            Some(value)
        } else {
            while self.internals.len() > 0 {
                let (node, index) = self.internals.pop().unwrap();
                if let Node::Branch(branch) = node {
                    if index + 1 < branch.children.len() {
                        let child = &branch.children[index + 1];
                        self.internals.push((node, index + 1));
                        self.internals.push((child, 0));
                    } else {
                        continue;
                    }

                    while let (Node::Branch(branch), _) = self.internals.last().unwrap() {
                        let child = &branch.children[0];
                        self.internals.push((child, 0));
                    }

                    let (leaf, _) = self.internals.pop().unwrap();

                    self.leaf = leaf;
                    self.index = 0;

                    return self.next();
                }
            }

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition() {
        let p = vec![5, 12];
        println!("{}", p.partition_point(|&n| n <= 4));

        let p = vec![10, 20, 40];
        println!("{}", p.partition_point(|&n| n <= 9));
        println!("{}", p.partition_point(|&n| n <= 10));
        println!("{}", p.partition_point(|&n| n < 11));
    }

    #[test]
    fn test_change_tree_prepend() {
        let mut tree = ChangeTree::new();
        tree.insert(0, 0);
        tree.insert(0, 1);
        tree.insert(0, 2);
        tree.insert(0, 3);

        assert_eq!(tree.size(), 4);

        if let Some(iter) = tree.iter_from(0) {
            assert_eq!(iter.collect::<Vec<_>>().clone(), vec![&3, &2, &1, &0]);
        }
    }

    #[test]
    fn test_change_tree_append() {
        let mut tree = ChangeTree::new();
        tree.insert(0, 0);
        tree.insert(1, 1);
        tree.insert(2, 2);
        tree.insert(3, 3);

        assert_eq!(tree.size(), 4);

        if let Some(iter) = tree.iter_from(0) {
            assert_eq!(iter.collect::<Vec<_>>().clone(), vec![&0, &1, &2, &3]);
        }
    }

    #[test]
    fn test_change_tree_insert_random() {
        let mut tree = ChangeTree::new();
        for k in 0..5 {
            tree.insert(k, k);
        }

        assert_eq!(tree.size(), 5);
        // tree.print();
        tree.insert(2, 6);

        assert_eq!(tree.index_of(&6).unwrap(), 2);
        assert_eq!(tree.index_of(&2).unwrap(), 3);
        assert_eq!(tree.at_index(3).unwrap(), &2);

        // tree.print();

        if let Some(iter) = tree.iter_from(2) {
            assert_eq!(iter.collect::<Vec<_>>().clone(), vec![&6, &2, &3, &4]);
        }
    }

    #[test]
    fn test_change_tree_insert_random1() {
        for order in 2..5 {
            let mut tree = ChangeTree::with_order(order);
            for k in 0..5 {
                tree.insert(k, k);
            }

            assert_eq!(tree.size(), 5);
            // tree.print();
            tree.insert(2, 6);

            assert_eq!(tree.index_of(&6).unwrap(), 2);
            assert_eq!(tree.index_of(&2).unwrap(), 3);
            assert_eq!(tree.at_index(3).unwrap(), &2);

            // tree.print();

            if let Some(iter) = tree.iter_from(2) {
                assert_eq!(iter.collect::<Vec<_>>().clone(), vec![&6, &2, &3, &4]);
            }
        }
    }

    #[test]
    fn test_change_tree_insert_random2() {
        let mut tree = ChangeTree::with_order(6);
        for k in 0..50 {
            tree.insert(k, k);
        }

        tree.insert(10, 51);

        assert_eq!(tree.index_of(&10).unwrap(), 11);

        tree.insert(20, 52);

        assert_eq!(tree.index_of(&20).unwrap(), 22);

        // tree.print();

        // if let Some(iter) = tree.iter_from(20) {
        //     println!("{:?}", iter.collect::<Vec<_>>().clone());
        // }
    }
}
