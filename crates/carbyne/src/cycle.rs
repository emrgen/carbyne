use crate::id::WithId;
use crate::Type;

/// check if the parenting relationship between `parent` and `child` creates a cycle
pub(crate) fn creates_cycle(parent: &Type, child: &Type) -> bool {
    // moving child to higher level in the tree does not create a cycle
    if child.depth() >= parent.depth() {
        return false;
    }

    let child_id = child.id();
    // if the child is already a parent of the parent, it will create a cycle
    let mut cursor = parent.clone();
    while let Some(p) = cursor.parent() {
        if p.id().eq(&child_id) {
            return true;
        }
        cursor = p;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Doc;

    #[test]
    fn test_nested_cycle_detected() {
        let doc = Doc::default();
        let root = doc.list();
        doc.set("root", root.clone());
        let mid = doc.list();
        let leaf = doc.list();
        root.append(mid.clone());
        mid.append(leaf.clone());

        let root_t: Type = root.into();
        let leaf_t: Type = leaf.into();
        // moving root under leaf would create a cycle (leaf is a descendant of root)
        assert!(creates_cycle(&leaf_t, &root_t));
        // moving leaf under root does not create a cycle (root is an ancestor, not descendant)
        assert!(!creates_cycle(&root_t, &leaf_t));
    }
}
