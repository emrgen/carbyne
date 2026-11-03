use crate::Id;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub(crate) struct NTree {
    root: NTreeNode,
    nodes: HashMap<Id, NTreeNode>,
}

impl NTree {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // insert a node into the tree
    pub(crate) fn insert(&mut self, id: Id, parent_id: Id, side: NodeSide) -> Result<(), String> {
        // check if the node already exists
        if self.nodes.contains_key(&id) {
            return Err(format!("Node {} already exists", id));
        }

        // create the new node
        self.nodes.insert(id, NTreeNode::new(id));

        // get the parent node
        let parent_node = self
            .nodes
            .get_mut(&parent_id)
            .ok_or_else(|| format!("Parent node {} not found", parent_id))?;

        match side {
            NodeSide::Left => {
                // insert in sorted order by id
                // let pos = parent_node.left_children.binary_search(&id)?;
                // match pos {
                //     Ok(_) => panic!("Node already exists"),
                //     Err(pos) => parent_node.left_children.insert(pos, id),
                // }
            }
            NodeSide::Right => {
                // insert in sorted order by id
                // let pos = parent_node.right_children.binary_search(&id)?;
                // match pos {
                //     Ok(_) => panic!("Node already exists"),
                //     Err(pos) => parent_node.right_children.insert(pos, id),
                // }
            }
        }

        Ok(())
    }

    // remove a node from the tree
    pub(crate) fn remove(&mut self, id: Id, parent_id: Id, side: NodeSide) -> Result<(), String> {
        let mut parent = self
            .nodes
            .get_mut(&parent_id)
            .ok_or_else(|| format!("Parent node {} not found", parent_id))?;

        match side {
            NodeSide::Left => {
                parent.left_children.retain(|&child_id| child_id != id);
            }
            NodeSide::Right => {
                parent.right_children.retain(|&child_id| child_id != id);
            }
        }

        // remove the node
        self.nodes
            .remove(&id)
            .ok_or_else(|| format!("Node {} not found", id))?;

        Ok(())
    }

    // return the first leaf node in the tree
    pub(crate) fn first_leaf(&self) -> Option<Id> {
        let mut current = self.root.id;
        while let Some(node) = self.nodes.get(&current) {
            if node.left_children.is_empty() {
                return Some(current);
            }

            current = node.left_children.first().cloned().unwrap();
        }

        None
    }

    // return the last leaf node in the tree
    pub(crate) fn last_leaf(&self) -> Option<Id> {
        let mut current = self.root.id;
        while let Some(node) = self.nodes.get(&current) {
            if node.right_children.is_empty() {
                return Some(current);
            }

            current = node.right_children.last().cloned().unwrap();
        }

        None
    }
}

// TODO: left_children and right_children will have multiple children only when there is a conflict
// if there is no conflict, left_children and right_children will have only one child, in that case may be having a vector is overkill.
#[derive(Debug, Clone, Default)]
pub(crate) struct NTreeNode {
    id: Id,
    left_children: Vec<Id>,
    right_children: Vec<Id>,
}

impl NTreeNode {
    pub(crate) fn new(id: Id) -> Self {
        Self {
            id,
            left_children: Vec::new(),
            right_children: Vec::new(),
        }
    }
}

// placement of children wrt the parent
enum NodeSide {
    Left,
    Right,
}
