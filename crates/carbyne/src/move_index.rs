use crate::cycle::creates_cycle;
use crate::id::{Id, WithId};
use crate::types::Type;
use hashbrown::HashMap;

/// MoveIndex owns the per-target mover stack for Kleppmann-style move
/// operations: for each moved item, the stack of Move items that have
/// targeted it, ordered by causal (DAG) order, with the top of the
/// stack being the currently-active mover.
///
/// This consolidates bookkeeping that used to be duplicated across three
/// call sites (NList::move_to for local moves, and two separate replay
/// loops in Doc::apply for undo/redo and remote integration), each of
/// which re-implemented its own partial understanding of "integrate a
/// mover": mark the target moved/unmoved, push/pop the stack, and (for
/// the two Doc::apply paths) skip the mover if its move would create a
/// cycle. The duplication had already drifted: the redo path panicked
/// via `_ => todo!()` on a missing target/parent while the remote-apply
/// path silently skipped the same case.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub(crate) struct MoveIndex {
    // the top most move is the last one which is active
    moves: HashMap<Id, Vec<Type>>,
}

impl MoveIndex {
    /// Integrate a mover against its target's new parent, used by every
    /// move-integration path (local move_to, redo, remote apply). Skips
    /// the mover - leaving the target's visibility unchanged - if moving
    /// under `new_parent` would create a cycle. Returns whether the
    /// mover was integrated.
    pub(crate) fn integrate(&mut self, target: &Type, new_parent: &Type, mover: Type) -> bool {
        if creates_cycle(new_parent, target) {
            return false;
        }

        target.item_ref().mark_moved();
        self.push(target.id(), mover);
        true
    }

    /// Push a mover onto the target's stack without a cycle check, for
    /// callers that already validated the move (e.g. NList::move_to,
    /// which checks the cycle against `self` before the mover item
    /// exists to construct a Type for it).
    pub(crate) fn push(&mut self, target_id: Id, mover: Type) {
        let entry = self.moves.entry(target_id).or_default();
        // mark the last mover as moved so that it will be treated as an invisible item
        if let Some(last) = entry.last() {
            last.item_ref().mark_moved();
        }

        mover.item_ref().mark_active();
        entry.push(mover);
    }

    /// Remove the last mover for the given target id.
    /// The new last mover after removal is marked as unmoved; if the
    /// stack becomes empty, `unmark_target` (called by the caller, since
    /// finding the target item requires DocStore) should be used.
    pub(crate) fn pop(&mut self, target_id: Id, mover: &Type) {
        let mover_id = mover.id();

        self.moves.entry(target_id).and_modify(|movers| {
            movers.retain(|x| x.id() != mover_id);
            if let Some(last) = movers.last() {
                last.item_ref().unmark_moved();
            }
        });

        mover.item_ref().mark_inactive();
    }

    /// True if the target has no movers left on its stack after a pop.
    /// Returns false if the target was never moved at all (no entry) -
    /// matching the original DocStore semantics, where that case means
    /// there is nothing to unmark, not "already unmoved".
    pub(crate) fn is_empty_for(&self, target_id: &Id) -> bool {
        self.moves.get(target_id).map_or(false, |v| v.is_empty())
    }
}
