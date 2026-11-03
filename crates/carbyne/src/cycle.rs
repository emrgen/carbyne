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
    while let Some(parent) = parent.parent() {
        if parent.id().eq(&child_id) {
            return true;
        }
    }

    false
}
