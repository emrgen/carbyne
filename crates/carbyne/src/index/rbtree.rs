use fractional_index::FractionalIndex;

use crate::item::WithIndex;
use crate::Type;
#[derive(Clone, Debug, Default)]
pub(crate) struct IndexTree {
    root: Option<Box<TreeNode>>,
}

#[derive(Debug, Clone)]
pub(crate) struct TreeNode {
    item: Type,
    left: Option<Box<TreeNode>>,
    right: Option<Box<TreeNode>>,
    color: Color,
    deleted: bool,
    left_count: usize,
    right_count: usize,
}

impl TreeNode {
    pub(crate) fn new(item: Type) -> Self {
        Self {
            item,
            left: None,
            right: None,
            color: Color::Red,
            deleted: false,
            left_count: 0,
            right_count: 0,
        }
    }

    fn index(&self) -> FractionalIndex {
        self.item.index()
    }

    fn is_deleted(&self) -> bool {
        self.deleted
    }

    fn update_counts(&mut self) {
        self.left_count = Self::count(&self.left);
        self.right_count = Self::count(&self.right);
    }

    fn count(node: &Option<Box<TreeNode>>) -> usize {
        match node {
            Some(n) => {
                let left_count = Self::count(&n.left);
                let right_count = Self::count(&n.right);
                left_count + right_count + if n.deleted { 0 } else { 1 }
            }
            None => 0,
        }
    }

    fn size(&self) -> usize {
        self.left_count + self.right_count + if self.deleted { 0 } else { 1 }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub(crate) enum Color {
    Red,
    Black,
}

// Red Black Tree implementation for FractionalIndex to store items
impl IndexTree {
    pub(crate) fn new() -> Self {
        Self { root: None }
    }

    pub fn size(&self) -> usize {
        match &self.root {
            Some(n) => n.size(),
            None => 0,
        }
    }

    pub fn insert(&mut self, value: Type) {
        self.root = Some(Self::insert_node(self.root.take(), value));
        if let Some(ref mut node) = self.root {
            node.color = Color::Black;
        }
    }

    // search for item in tree
    pub fn contains(&self, value: &Type) -> bool {
        Self::contains_node(&self.root, value)
    }

    fn contains_node(node: &Option<Box<TreeNode>>, value: &Type) -> bool {
        match node {
            Some(n) => {
                if value.index() == n.index() {
                    !n.deleted
                } else if value.index() < n.index() {
                    Self::contains_node(&n.left, value)
                } else {
                    Self::contains_node(&n.right, value)
                }
            }
            None => false,
        }
    }

    fn undelete(&mut self, value: &Type) {
        self.root = Self::undelete_node(self.root.take(), value);
    }

    fn undelete_node(node: Option<Box<TreeNode>>, value: &Type) -> Option<Box<TreeNode>> {
        match node {
            Some(mut n) => {
                if value.index() < n.index() {
                    n.left = Self::undelete_node(n.left.take(), value);
                } else if value.index() > n.index() {
                    n.right = Self::undelete_node(n.right.take(), value);
                } else {
                    n.deleted = false;
                }
                Some(n)
            }
            None => None,
        }
    }

    // remove item from tree
    pub fn remove(&mut self, value: &Type) {
        if self.root.is_none() {
            return;
        }

        self.root = Some(Self::remove_node(self.root.take(), value));

        if let Some(ref mut node) = self.root {
            node.color = Color::Black;
        }
    }

    fn remove_node(node: Option<Box<TreeNode>>, value: &Type) -> Box<TreeNode> {
        let mut node = node.unwrap();
        if value.index() < node.index() {
            if node.left.is_none() {
                return node;
            }

            if !Self::is_red(&node.left) && !Self::is_red(&node.left.as_ref().unwrap().left) {
                node = Self::move_red_left(node);
            }
            node.left = Some(Self::remove_node(node.left.take(), value));
        } else {
            if Self::is_red(&node.left) {
                node = Self::rotate_right(node);
            }
            if value.index() == node.index() && node.right.is_none() {
                return node;
            }
            if !Self::is_red(&node.right) && !Self::is_red(&node.right.as_ref().unwrap().left) {
                node = Self::move_red_right(node);
            }
            if value.index() == node.index() {
                let mut min_node = Self::min(node.right.take().unwrap());
                node.item = min_node.item;
                node.right = Self::delete_min(node.right.take());
            } else {
                node.right = Some(Self::remove_node(node.right.take(), value));
            }
        }

        Self::fix_up(node)
    }

    fn delete_min(node: Option<Box<TreeNode>>) -> Option<Box<TreeNode>> {
        let mut node = node.unwrap();
        if node.left.is_none() {
            return None;
        }
        if !Self::is_red(&node.left) && !Self::is_red(&node.left.as_ref().unwrap().left) {
            node = Self::move_red_left(node);
        }
        node.left = Self::delete_min(node.left.take());
        Some(Self::fix_up(node))
    }

    fn min(mut node: Box<TreeNode>) -> Box<TreeNode> {
        while let Some(left) = node.left.take() {
            node = left;
        }
        node
    }

    fn move_red_left(mut node: Box<TreeNode>) -> Box<TreeNode> {
        Self::flip_colors(&mut node);
        if Self::is_red(&node.right.as_ref().unwrap().left) {
            node.right = Some(Self::rotate_right(node.right.take().unwrap()));
            node = Self::rotate_left(node);
            Self::flip_colors(&mut node);
        }
        node
    }

    fn move_red_right(mut node: Box<TreeNode>) -> Box<TreeNode> {
        Self::flip_colors(&mut node);
        if Self::is_red(&node.left.as_ref().unwrap().left) {
            node = Self::rotate_right(node);
            Self::flip_colors(&mut node);
        }
        node
    }

    fn fix_up(mut node: Box<TreeNode>) -> Box<TreeNode> {
        if Self::is_red(&node.right) {
            node = Self::rotate_left(node);
        }
        if Self::is_red(&node.left) && Self::is_red(&node.left.as_ref().unwrap().left) {
            node = Self::rotate_right(node);
        }
        if Self::is_red(&node.left) && Self::is_red(&node.right) {
            Self::flip_colors(&mut node);
        }
        node
    }

    pub fn index_of(&self, value: &Type) -> usize {
        Self::find_index_of(&self.root, value)
    }

    fn find_index_of(node: &Option<Box<TreeNode>>, value: &Type) -> usize {
        match node {
            Some(n) => {
                return if value.index() > n.index() {
                    n.left_count
                        + Self::find_index_of(&n.right, value)
                        + if n.deleted { 0 } else { 1 }
                } else if value.index() < n.index() {
                    Self::find_index_of(&n.left, value)
                } else {
                    n.left_count
                }
            }
            None => 0,
        }
    }

    pub fn at_index(&self, index: u32) -> Option<&Type> {
        Self::find_at_index(&self.root, index as usize)
    }

    fn find_at_index(node: &Option<Box<TreeNode>>, index: usize) -> Option<&Type> {
        match node {
            Some(n) => {
                let left_count = n.left_count;
                return if left_count == index && !n.deleted {
                    Some(&n.item)
                } else if left_count > index {
                    Self::find_at_index(&n.left, index)
                } else {
                    Self::find_at_index(
                        &n.right,
                        index - left_count - if n.deleted { 0 } else { 1 },
                    )
                };
            }
            None => None,
        }
    }

    // mark item as deleted
    fn delete(&mut self, value: &Type) {
        self.root = Self::delete_node(self.root.take(), value);
        if let Some(ref mut node) = self.root {
            node.update_counts();
        }
    }

    fn delete_node(mut node: Option<Box<TreeNode>>, value: &Type) -> Option<Box<TreeNode>> {
        match node {
            Some(mut n) => {
                if value.index() < n.index() {
                    n.left = Self::delete_node(n.left.take(), value);
                } else if value.index() > n.index() {
                    n.right = Self::delete_node(n.right.take(), value);
                } else {
                    n.deleted = true;
                }

                n.update_counts();

                Some(n)
            }
            None => None,
        }
    }

    fn insert_node(mut node: Option<Box<TreeNode>>, value: Type) -> Box<TreeNode> {
        match node {
            Some(mut n) => {
                if value.index() < n.index() {
                    n.left = Some(Self::insert_node(n.left.take(), value));
                    // println!("insert left: {:?}", n.item.content());
                } else {
                    n.right = Some(Self::insert_node(n.right.take(), value));
                    // println!("insert right: {:?}", n.item.content());
                }

                // n.update_counts();
                //
                // n

                let mut node = Self::fix_violations(n);
                node.update_counts();

                node
            }
            None => Box::new(TreeNode::new(value)),
        }
    }

    fn fix_violations(mut node: Box<TreeNode>) -> Box<TreeNode> {
        if Self::is_red(&node.right) && !Self::is_red(&node.left) {
            node = Self::rotate_left(node);
        }

        if Self::is_red(&node.left) && Self::is_red(&node.left.as_ref().unwrap().left) {
            node = Self::rotate_right(node);
        }

        if Self::is_red(&node.left) && Self::is_red(&node.right) {
            Self::flip_colors(&mut node);
        }

        node
    }

    fn is_red(node: &Option<Box<TreeNode>>) -> bool {
        match node {
            Some(n) => n.color == Color::Red,
            None => false,
        }
    }

    fn rotate_left(mut node: Box<TreeNode>) -> Box<TreeNode> {
        let mut new_node = node.right.take().unwrap();

        node.right = new_node.left.take();
        node.update_counts();

        new_node.left = Some(node);
        new_node.color = new_node.left.as_ref().unwrap().color;
        new_node.left.as_mut().unwrap().color = Color::Red;
        new_node.update_counts();

        new_node
    }

    fn rotate_right(mut node: Box<TreeNode>) -> Box<TreeNode> {
        let mut new_node = node.left.take().unwrap();

        node.left = new_node.right.take();
        node.update_counts();

        new_node.right = Some(node);
        new_node.color = new_node.right.as_ref().unwrap().color;
        new_node.right.as_mut().unwrap().color = Color::Red;
        new_node.update_counts();

        new_node
    }

    fn flip_colors(node: &mut Box<TreeNode>) {
        node.color = match node.color {
            Color::Red => Color::Black,
            Color::Black => Color::Red,
        };
        if let Some(ref mut left) = node.left {
            left.color = match left.color {
                Color::Red => Color::Black,
                Color::Black => Color::Red,
            };
        }
        if let Some(ref mut right) = node.right {
            right.color = match right.color {
                Color::Red => Color::Black,
                Color::Black => Color::Red,
            };
        }
    }

    pub fn iter(&self) -> TreeIter {
        TreeIter::new(&self.root)
    }

    // print tree with dashes to show the tree structure
    pub fn print_tree(&self) {
        self.print_tree_node(&self.root, 0);
    }

    fn print_tree_node(&self, node: &Option<Box<TreeNode>>, level: usize) {
        match node {
            Some(n) => {
                self.print_tree_node(&n.right, level + 1);
                for _ in 0..level {
                    print!("--");
                }
                println!("{:?}", n.item.content());
                self.print_tree_node(&n.left, level + 1);
            }
            None => {}
        }
    }
}

pub(crate) struct TreeIter<'a> {
    stack: Vec<&'a Box<TreeNode>>,
}

impl<'a> TreeIter<'a> {
    pub(crate) fn new(root: &'a Option<Box<TreeNode>>) -> Self {
        let mut stack = Vec::new();
        let mut current = root;
        while let Some(node) = current {
            stack.push(node);
            current = &node.left;
        }

        Self { stack }
    }
}

impl<'a> Iterator for TreeIter<'a> {
    type Item = &'a TreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        let mut current = &node.right;
        while let Some(n) = current {
            self.stack.push(n);
            current = &n.left;
        }

        Some(node)
    }
}

#[cfg(test)]
mod test {
    use fractional_index::FractionalIndex;
    use rand::prelude::SliceRandom;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    use crate::index::rbtree::IndexTree;
    use crate::item::WithIndex;
    use crate::{Doc, Type};

    #[test]
    fn test_insert_node() {
        let mut tree = IndexTree::new();
        let doc = Doc::default();

        let s1: Type = doc.string("a").into();
        let s2: Type = doc.string("b").into();
        let s3: Type = doc.string("c").into();
        let s4: Type = doc.string("d").into();

        s1.item_ref().borrow_mut().index = (FractionalIndex::default());
        s2.item_ref().borrow_mut().index = (FractionalIndex::new_after(&s1.index()));
        s3.item_ref().borrow_mut().index = FractionalIndex::new_after(&s2.index());
        s4.item_ref().borrow_mut().index =
            FractionalIndex::new_between(&s2.index(), &s3.index()).unwrap();

        tree.insert(s1.clone());
        tree.insert(s2.clone());
        tree.insert(s3.clone());
        tree.insert(s4.clone());

        assert_eq!(tree.contains(&s1), true);
        assert_eq!(tree.contains(&s2), true);
        assert_eq!(tree.contains(&s3), true);

        // for i in tree.iter() {
        //     println!(
        //         "item: {:?}, index: {:?}, left: {:?}",
        //         i.item.content(),
        //         i.index(),
        //         tree.left_count(&i.item)
        //     );
        // }
        //
        // tree.print_tree();

        assert_eq!(tree.index_of(&s1), 0);
        assert_eq!(tree.index_of(&s2), 1);
        assert_eq!(tree.index_of(&s4), 2);
        assert_eq!(tree.index_of(&s3), 3);
    }

    #[test]
    fn test_delete_node() {
        let mut tree = IndexTree::new();
        let doc = Doc::default();

        let s1: Type = doc.string("a").into();
        let s2: Type = doc.string("b").into();
        let s3: Type = doc.string("c").into();
        let s4: Type = doc.string("d").into();
        let s5: Type = doc.string("e").into();

        s1.item_ref().borrow_mut().index = (FractionalIndex::default());
        s2.item_ref().borrow_mut().index = (FractionalIndex::new_after(&s1.index()));
        s3.item_ref().borrow_mut().index = (FractionalIndex::new_after(&s2.index()));
        s4.item_ref().borrow_mut().index =
            FractionalIndex::new_between(&s2.index(), &s3.index()).unwrap();

        s5.item_ref().borrow_mut().index = (FractionalIndex::new_after(&s3.index()));

        tree.insert(s1.clone());
        tree.insert(s2.clone());
        tree.insert(s3.clone());
        tree.insert(s4.clone());
        tree.insert(s5.clone());

        tree.delete(&s4.clone());
        tree.delete(&s2.clone());

        // for i in tree.iter() {
        //     println!(
        //         "item: {:?}, index: {:?}, left: {:?}",
        //         i.item.content(),
        //         i.index(),
        //         tree.left_count(&i.item)
        //     );
        // }
        //
        // tree.print_tree();

        assert_eq!(tree.contains(&s2), false);
        assert_eq!(tree.contains(&s4), false);
        assert_eq!(tree.index_of(&s1), 0);
        assert_eq!(tree.index_of(&s3), 1);
    }

    #[test]
    fn test_insert_1000_nodes() {
        let mut tree = IndexTree::new();
        let doc = Doc::default();

        let mut nodes = Vec::new();
        let mut prev: Option<Type> = None;

        for i in 0..10 {
            let s: Type = doc.string(i.to_string()).into();
            if let Some(p) = prev {
                s.item_ref().borrow_mut().index = (FractionalIndex::new_after(&p.index()));
            } else {
                s.item_ref().borrow_mut().index = (FractionalIndex::default());
            }

            nodes.push(s.clone());

            prev = Some(s.clone());
        }

        // seed the rng
        let mut rng = ChaCha8Rng::seed_from_u64(2);
        nodes.shuffle(&mut rng);

        for n in &nodes {
            tree.insert(n.clone());
        }

        tree.delete(&nodes[0]);
        tree.delete(&nodes[1]);
        tree.delete(&nodes[2]);

        let visible = tree.iter().filter(|n| !n.is_deleted()).collect::<Vec<_>>();

        // for n in &visible {
        //     println!(
        //         "node: {:?}, index: {:?}, left: {:?}; right: {:?}, color: {:?}",
        //         n.item.content(),
        //         tree.index_of(&n.item),
        //         n.left_count,
        //         n.right_count,
        //         n.color
        //     );
        // }

        // tree.print_tree();

        let item = tree.at_index(3);
        assert_eq!(item.unwrap().content().to_json(), "5".to_string());

        let item = tree.at_index(5);
        assert_eq!(item.unwrap().content().to_json(), "7".to_string());

        let item = tree.at_index(6);
        assert_eq!(item.unwrap().content().to_json(), "9".to_string());
    }
}
