use ptree::{print_tree, TreeBuilder};
use std::fmt::{Debug, Display};
use std::io::Write;
use std::mem;

#[derive(Debug, Clone)]
struct Leaf<K, V> {
    values: Vec<V>,
    keys: Vec<K>,
    order: usize, // The order of the leaf node
}

impl<K: Ord + Clone + Debug, V: Clone + Debug> Leaf<K, V> {
    fn new(order: usize) -> Self {
        Leaf {
            values: Vec::with_capacity(order),
            keys: Vec::with_capacity(order),
            order,
        }
    }

    fn insert(&mut self, key: K, value: V) -> Option<(K, Node<K, V>)> {
        let pos = self
            .keys
            .binary_search_by_key(&key, |k| k.clone())
            .unwrap_or_else(|e| e);

        // If leaf is full, we need to handle by splitting the leaf node
        if self.keys.len() == self.keys.capacity() {
            Some(self.insert_and_split(pos, key, value))
        } else {
            // if the key is already present, overwrite the value
            if self.keys.get(pos).map_or(false, |v| v == &key) {
                self.values[pos] = value;
            } else {
                self.keys.insert(pos, key);
                self.values.insert(pos, value);
            }

            None
        }
    }

    // returns the (split key and new node)
    fn insert_and_split(&mut self, pos: usize, key: K, value: V) -> (K, (Node<K, V>)) {
        let mid = self.keys.len() / 2;

        let mut new_leaf = Leaf::new(self.order);
        new_leaf.keys.extend(self.keys.drain(mid..));
        new_leaf.values.extend(self.values.drain(mid..));

        if pos < mid {
            self.keys.insert(pos, key);
            self.values.insert(pos, value);
        } else {
            new_leaf.keys.insert(pos - mid, key);
            new_leaf.values.insert(pos - mid, value);
        }

        let min_key = new_leaf.min_key().unwrap().clone();

        (min_key, Node::new_leaf(new_leaf))
    }

    fn min_key(&self) -> Option<&K> {
        self.keys.first()
    }

    fn max_key(&self) -> Option<&K> {
        self.keys.last()
    }

    fn size(&self) -> usize {
        self.values.len()
    }

    fn print(&self, tree: &mut TreeBuilder) {
        let mut string_builder = String::new();
        string_builder.push_str(&format!("Leaf: {} => ", self.keys.capacity()));
        for (key, value) in self.keys.iter().zip(&self.values) {
            string_builder.push_str(&format!("{:?}: {:?}, ", key, value));
        }
        tree.begin_child(string_builder).end_child();
    }
}

#[derive(Debug, Clone)]
struct Branch<K, V> {
    keys: Vec<K>,
    children: Vec<Node<K, V>>,
    order: usize,
    total: usize, // pre-computed size for performance
}

impl<K: Ord + Clone + Debug, V: Clone + Debug> Branch<K, V> {
    fn new(order: usize) -> Self {
        Branch {
            keys: Vec::with_capacity(order),
            children: Vec::with_capacity(order + 1),
            order,
            total: 0,
        }
    }

    fn insert(&mut self, key: K, value: V) -> Option<(K, Node<K, V>)> {
        let pos = self.keys.partition_point(|k| k <= &key);

        if let Some(child) = self.children.get_mut(pos) {
            if let Some((new_key, new_node)) = child.insert(key, value) {
                if self.is_full() {
                    // new key needs to be inserted at pos+1 position
                    return Some(self.insert_and_split(pos, new_key, new_node));
                } else {
                    // If the child node split, we need to handle the new node
                    self.keys.insert(pos, new_key);
                    self.children.insert(pos + 1, new_node);
                    self.total += 1;
                }
            } else {
                self.total += 1;
            }
        }

        None
    }

    fn insert_and_split(
        &mut self,
        pos: usize,
        new_key: K,
        new_node: Node<K, V>,
    ) -> (K, Node<K, V>) {
        let mid = self.keys.len() / 2;

        let mut new_branch = Branch::new(self.order);

        self.keys.insert(pos, new_key);
        self.children.insert(pos + 1, new_node);

        let mid_key = self.keys[mid].clone();

        new_branch.keys.extend(self.keys[mid + 1..].iter().cloned());
        new_branch
            .children
            .extend(self.children[mid + 1..].iter().cloned());

        self.keys.truncate(mid);
        self.children.truncate(mid + 1);

        // keep the capacity of the self branch same as new_branch
        self.keys.shrink_to(new_branch.keys.capacity());
        self.children.shrink_to(new_branch.children.capacity());

        new_branch.update_count();
        self.update_count();

        (mid_key, Node::new_branch(new_branch))
    }

    fn update_count(&mut self) {
        self.total = self.children.iter().map(|child| child.size()).sum();
    }

    fn is_full(&self) -> bool {
        self.keys.len() == self.keys.capacity()
    }

    fn min_key(&self) -> Option<&K> {
        self.children.first().map(|child| child.min_key()).flatten()
    }

    fn max_key(&self) -> Option<&K> {
        self.children.last().map(|child| child.max_key()).flatten()
    }

    fn size(&self) -> usize {
        self.total
    }

    fn print(&self, tree: &mut TreeBuilder) {
        tree.begin_child(format!("Branch: {}", self.keys.len()));
        for (i, child) in self.children.iter().enumerate() {
            if i > 0 {
                let key = self.keys.get(i - 1);
                tree.begin_child(format!("Key: {:?}", key.unwrap()));
            } else {
                tree.begin_child(format!("Key: {}", "#"));
            }
            child.print(tree);
            tree.end_child();
        }
        tree.end_child();
    }
}

#[derive(Debug, Clone)]
enum Node<K, T> {
    Leaf(Leaf<K, T>),
    Branch(Branch<K, T>),
}

fn binary_search<K: Ord + Debug>(keys: &[K], key: &K) -> usize {
    keys.partition_point(|k| k <= key)
}

impl<K: Ord + Clone + Debug, V: Clone + Debug> Node<K, V> {
    fn new_leaf(leaf: Leaf<K, V>) -> Self {
        Node::Leaf(leaf)
    }

    fn new_branch(branch: Branch<K, V>) -> Self {
        Node::Branch(branch)
    }

    // Insert a key-value pair into the node
    // Returns Some((key, new_node)) if the node was split
    fn insert(&mut self, key: K, value: V) -> Option<(K, Node<K, V>)> {
        match self {
            Node::Leaf(leaf) => leaf.insert(key, value),
            Node::Branch(branch) => branch.insert(key, value),
        }
    }

    fn find(&self, key: &K) -> Option<&V> {
        match self {
            Node::Leaf(leaf) => {
                let mut tree = TreeBuilder::new(format!("Leaf: {:?}", self.size()));
                leaf.print(&mut tree);
                print_tree(&tree.build()).unwrap();

                let pos = leaf.keys.binary_search(key).unwrap_or_else(|e| e);
                if leaf.keys.len() > pos && leaf.keys.get(pos) == Some(key) {
                    leaf.values.get(pos)
                } else {
                    None
                }
            }
            Node::Branch(branch) => {
                let mut tree = TreeBuilder::new(format!("Branch: {:?}", self.size()));
                branch.print(&mut tree);
                print_tree(&tree.build()).unwrap();

                let pos = branch.keys.partition_point(|k| k <= key);

                // let pos = branch.keys.binary_search(key).unwrap_or_else(|e| e);
                println!(
                    "key: {:?}, pos: {}, keys: {:?}, children: {}",
                    key,
                    pos,
                    branch.keys,
                    branch.children.len()
                );

                if pos < branch.children.len() {
                    branch.children[pos].find(key)
                } else {
                    None
                }
            }
        }
    }

    fn find_mut(&mut self, key: &K) -> Option<&mut V> {
        match self {
            Node::Leaf(leaf) => {
                let pos = leaf.keys.binary_search(key).ok()?;
                Some(&mut leaf.values[pos])
            }
            Node::Branch(branch) => {
                let pos = branch.keys.binary_search(key).unwrap_or_else(|e| e);
                if pos < branch.children.len() {
                    branch.children[pos].find_mut(key)
                } else {
                    None
                }
            }
        }
    }

    fn contains(&self, key: &K) -> bool {
        match self {
            Node::Leaf(leaf) => leaf.keys.contains(key),
            Node::Branch(branch) => {
                let pos = branch.keys.binary_search(key).unwrap_or_else(|e| e);
                if pos < branch.children.len() {
                    branch.children[pos].contains(key)
                } else {
                    false
                }
            }
        }
    }

    fn index_of(&self, key: &K) -> Option<usize> {
        match self {
            Node::Leaf(leaf) => leaf.keys.binary_search(key).ok(),
            Node::Branch(branch) => {
                let pos = branch.keys.partition_point(|k| k <= key);
                if pos < branch.children.len() {
                    // calculate total value count in children[0..pos]
                    let total_size: usize = branch.children[0..pos]
                        .iter()
                        .map(|child| child.size())
                        .sum();

                    branch.children[pos].index_of(key).map(|i| i + total_size)
                } else {
                    None
                }
            }
        }
    }

    fn key_at_index(&self, index: usize) -> Option<&K> {
        match self {
            Node::Leaf(leaf) => leaf.keys.get(index),
            Node::Branch(branch) => {
                if index >= branch.size() {
                    return None;
                }

                let mut current_index = index;
                for (i, child) in branch.children.iter().enumerate() {
                    if i > 0 {
                        current_index -= branch.children[i - 1].size();
                    }

                    if current_index < child.size() {
                        return child.key_at_index(current_index);
                    }
                }

                None
            }
        }
    }

    /// Returns a reference to the value at the given index, if it exists.
    fn at_index(&self, index: usize) -> Option<&V> {
        match self {
            Node::Leaf(leaf) => leaf.values.get(index),
            Node::Branch(branch) => {
                if index >= branch.size() {
                    return None;
                }

                let mut current_index = index;
                for (i, child) in branch.children.iter().enumerate() {
                    if i > 0 {
                        current_index -= branch.children[i - 1].size();
                    }

                    if current_index < child.size() {
                        return child.at_index(current_index);
                    }
                }

                None
            }
        }
    }

    /// Returns a mutable reference to the value at the given index, if it exists.
    fn at_index_mut(&mut self, index: usize) -> Option<&mut V> {
        match self {
            Node::Leaf(leaf) => leaf.values.get_mut(index),
            Node::Branch(branch) => {
                if index >= branch.size() {
                    return None;
                }

                let mut current_index = index;
                let sizes = branch.children.iter().map(|c| c.size()).collect::<Vec<_>>();
                for (i, child) in branch.children.iter_mut().enumerate() {
                    if i > 0 {
                        current_index -= sizes[i - 1];
                    }

                    if current_index < child.size() {
                        return child.at_index_mut(current_index);
                    }
                }

                None
            }
        }
    }

    fn min_key(&self) -> Option<&K> {
        match self {
            Node::Leaf(leaf) => leaf.min_key(),
            Node::Branch(branch) => branch.min_key(),
        }
    }

    fn max_key(&self) -> Option<&K> {
        match self {
            Node::Leaf(leaf) => leaf.max_key(),
            Node::Branch(branch) => branch.max_key(),
        }
    }

    fn size(&self) -> usize {
        match self {
            Node::Leaf(leaf) => leaf.size(),
            Node::Branch(branch) => branch.size(),
        }
    }

    fn is_leaf(&self) -> bool {
        matches!(self, Node::Leaf(_))
    }

    fn is_branch(&self) -> bool {
        matches!(self, Node::Branch(_))
    }

    fn is_empty(&self) -> bool {
        match self {
            Node::Leaf(leaf) => leaf.keys.is_empty(),
            Node::Branch(branch) => branch.keys.is_empty(),
        }
    }

    fn is_full(&self) -> bool {
        match self {
            Node::Leaf(leaf) => leaf.keys.len() == leaf.keys.capacity(),
            Node::Branch(branch) => branch.is_full(),
        }
    }

    fn print(&self, tree: &mut TreeBuilder) {
        match self {
            Node::Leaf(leaf) => leaf.print(tree),
            Node::Branch(branch) => branch.print(tree),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BTree<K, V> {
    root: Node<K, V>,
    order: usize,
}

impl<K: Debug + Ord + Clone + Debug, V: Clone + Debug> BTree<K, V> {
    fn default() -> Self {
        BTree::new(10) // Default order is 2
    }

    pub(crate) fn new(order: usize) -> Self {
        BTree {
            root: Node::Leaf(Leaf::new(order)),
            order,
        }
    }

    // Insert a key-value pair into the B-Tree
    pub(crate) fn insert(&mut self, key: K, value: V) {
        if let Some((key, node)) = self.root.insert(key, value) {
            let mut new_root = Node::new_branch(Branch::new(self.order));
            let old_root = mem::replace(&mut self.root, new_root);
            match self.root {
                Node::Branch(ref mut branch) => {
                    branch.children.push(old_root);
                    branch.children.push(node);
                    branch.keys.push(key);
                    branch.update_count();
                }
                _ => unreachable!(),
            }
        }
    }

    pub(crate) fn remove(&mut self, key: &K) -> Option<V> {
        // Removal logic will go here
        // This is a placeholder for the actual implementation
        None
    }

    pub(crate) fn find(&self, key: &K) -> Option<&V> {
        self.root.find(key)
    }

    pub(crate) fn find_mut(&mut self, key: &K) -> Option<&mut V> {
        self.root.find_mut(key)
    }

    pub(crate) fn contains(&self, key: &K) -> bool {
        self.contains(key)
    }

    pub(crate) fn index_of(&self, key: &K) -> Option<usize> {
        self.root.index_of(key)
    }

    pub(crate) fn at_index(&self, index: usize) -> Option<&V> {
        self.root.at_index(index)
    }

    pub(crate) fn at_index_mut(&mut self, index: usize) -> Option<&mut V> {
        self.root.at_index_mut(index)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    pub(crate) fn size(&self) -> usize {
        self.root.size()
    }

    pub(crate) fn print(&self) {
        let mut tree = TreeBuilder::new(format!("BTree: {:?}", self.size()));
        self.root.print(&mut tree);
        print_tree(&tree.build()).unwrap();
    }

    pub(crate) fn iter(&self) -> EntryIter<K, V> {
        EntryIter::new(&self.root)
    }
}

pub(crate) struct EntryIter<'a, K: Ord + Clone + Debug, V: std::fmt::Debug> {
    internals: Vec<(&'a Node<K, V>, usize)>,
    leaf: &'a Node<K, V>,
    index: usize,
}

impl<'a, K: Ord + Clone + Debug, V: Debug> EntryIter<'a, K, V> {
    fn new(node: &'a Node<K, V>) -> Self {
        let mut internals = vec![(node, 0)];

        while let (Node::Branch(internal), _) = internals.last().unwrap() {
            let child = &internal.children[0];
            internals.push((child, 0));
        }

        let (leaf, _) = internals.pop().unwrap();

        EntryIter {
            internals,
            leaf,
            index: 0,
        }
    }
}

impl<'a, K: Ord + Clone + Debug, V: Debug + Clone> Iterator for EntryIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.leaf.size() {
            let value = self.leaf.at_index(self.index)?;
            let key = self.leaf.key_at_index(self.index)?;
            self.index += 1;

            Some((key, value))
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
    use rand::prelude::{SliceRandom, StdRng};
    use rand::{Rng, SeedableRng};

    #[test]
    fn test_btree_insert() {
        let mut btree = BTree::new(4);
        btree.insert(10, "A");
        btree.insert(20, "B");
        btree.insert(5, "C");

        assert_eq!(btree.size(), 3);
        assert!(!btree.is_empty());
    }

    #[test]
    fn test_leaf_split() {
        let mut leaf = Leaf::new(2);
        leaf.insert(10, "A");
        leaf.insert(20, "B");
        let new_leaf = leaf.insert(5, "C");
        assert!(new_leaf.is_some());
        assert_eq!(new_leaf.unwrap().1.size(), 1);
        assert_eq!(leaf.size(), 2);
    }

    #[test]
    fn test_branch_split() {
        let mut tree = BTree::new(2);
        tree.insert(10, "A");
        tree.insert(20, "B");
        assert!(tree.root.is_leaf());

        tree.insert(30, "C");
        assert!(tree.root.is_branch());
        assert_eq!(tree.size(), 3);

        tree.insert(5, "D");
        tree.insert(7, "E");
        tree.insert(6, "F");

        // split should occur here
        tree.insert(22, "G");

        // tree.print();

        assert_eq!(tree.find(&10), Some(&"A"));
        assert_eq!(tree.find(&5), Some(&"D"));
        assert_eq!(tree.find(&30), Some(&"C"));

        assert_eq!(tree.size(), 7);
        assert_eq!(tree.at_index(0), Some(&"D"));
        assert_eq!(tree.at_index(1), Some(&"F"));
        assert_eq!(tree.at_index(2), Some(&"E"));
        assert_eq!(tree.at_index(3), Some(&"A"));
        assert_eq!(tree.at_index(4), Some(&"B"));
        assert_eq!(tree.at_index(5), Some(&"G"));
        assert_eq!(tree.at_index(6), Some(&"C"));
    }

    #[test]
    fn test_btree_index_of() {
        for i in 0..1 {
            for order in 2..3 {
                let mut tree = BTree::new(order);

                let mut keys = (0..4).collect::<Vec<_>>();
                let sorted_keys = keys.clone();

                let mut rng = StdRng::seed_from_u64(i);
                // Shuffle the keys to ensure random order
                keys.shuffle(&mut rng);

                for k in &keys {
                    tree.insert(*k, *k);
                }

                // tree.print();

                // let index = tree.index_of(&3).unwrap();
                for k in &sorted_keys {
                    let index = tree.index_of(k).unwrap();
                    // println!("key: {}, index: {:?}", k, index);
                }
                let sorted = sorted_keys
                    .iter()
                    .map(|k| tree.index_of(k).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(sorted, sorted_keys);
            }
        }
    }

    #[test]
    fn test_binary_search() {
        let l1 = [1, 2, 3, 4, 5];
        assert_eq!(binary_search(&l1, &3), 3);
    }

    #[test]
    fn test_btree_iter() {
        for i in 0..100 {
            for order in 2..10 {
                let mut tree = BTree::new(order);

                let mut keys = (0..100).collect::<Vec<_>>();
                let sorted_keys = keys.clone();

                let mut rng = StdRng::seed_from_u64(i);
                // Shuffle the keys to ensure random order
                keys.shuffle(&mut rng);

                // Insert keys into the B-Tree
                for i in keys.iter() {
                    tree.insert(*i, i);
                    // tree.print();
                }

                let mut iter = tree.iter();
                let items = iter.map(|(k, _)| *k).collect::<Vec<_>>();

                assert_eq!(items.len(), keys.len());
                assert_eq!(items, sorted_keys);

                // println!("items: {:?}", items);
                // println!("sorted: {:?}", sorted_keys);
            }
        }
    }
}
