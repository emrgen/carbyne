use std::cmp::Ordering;

use crate::bimapid::ClientMap;
use crate::id::{Id, WithId};
use crate::item::Linked;
use crate::store::ClientStore;
use crate::types::Type;

// integrate an item into the list of items, resolving conflicts
pub(crate) fn integrate_yata<SS, SE>(
    client_map: &ClientMap,
    item: &Type,
    parent: &Type,
    start: Option<Type>,
    left: &mut Option<Type>,
    right: Option<Type>,
    set_start: SS,
    set_end: SE,
) -> Result<(i32), String>
where
    SS: FnOnce(Option<Type>),
    SE: FnOnce(Option<Type>),
{
    // let item: Type = ItemRef::new(data.into(), store.clone()).into();
    // print_yaml(&item);

    let left_conflict = || {
        let prev = item.left();
        let prev_id = prev.map(|p| p.id());
        let left_id = left.as_ref().map(|l| l.id());

        // if prev item's id is not equal to the items left origin id
        !Id::eq_opt(&prev_id, &left_id)
    };

    let right_conflict = || {
        let next = item.right();
        let next_id = next.map(|n| n.id());
        let right_id = right.as_ref().map(|r| r.id());

        // if next item's id is not equal to the items right origin id
        !Id::eq_opt(&next_id, &right_id)
    };

    let mut conflict: Option<Type> = None;
    if left.is_none() && right.is_none() || right_conflict() || left_conflict() {
        if let Some(left) = &left {
            conflict.clone_from(&left.right());
        } else {
            conflict.clone_from(&start);
        }
    }

    let mut counter = 0;
    {
        let mut conflict_items = ClientStore::default();
        let mut items_before_origin = ClientStore::default();

        let item_id = item.id();

        // println!("right: {:?}", right.as_ref().map(|r| r.id()));
        // println!("conflict: {:?}", conflict.as_ref().map(|r| r.id()));
        // resolve conflicts
        while conflict.is_some() && !Type::eq_opt(&conflict, &right) {
            // println!(
            //     "current conflict: {}",
            //     &conflict.as_ref().map(|c| c.id()).unwrap()
            // );
            // if right.is_some() {
            //     println!("right: {}", &right.as_ref().map(|c| c.id()).unwrap());
            // }

            counter += 1;
            let curr_conflict = conflict.as_ref().unwrap();
            let conflict_id = curr_conflict.id();

            if counter > 10_000_000 {
                println!(
                    "infinite loop: conflict: {}, right: {}",
                    conflict.as_ref().unwrap().id(),
                    right.as_ref().unwrap().id()
                );

                println!("right: {:?}", &curr_conflict.right_id());

                return Err("infinite loop".to_string());
            }

            items_before_origin.insert(conflict_id);
            conflict_items.insert(conflict_id);

            let conflict_left_id = conflict.as_ref().and_then(|c| c.left_id());
            let item_left_id = item.left_id();

            // println!("conflict_left_id: {:?}", conflict_left_id);
            // println!("item_left_id: {:?}", item_left_id);
            // println!("conflict: {:?}", curr_conflict.id());

            if Id::eq_opt(&conflict_left_id, &item_left_id) {
                if item_id.compare(&curr_conflict.id(), client_map) == Ordering::Greater {
                    // println!("->item id is greater than item conflict id");
                    left.clone_from(&conflict);
                    conflict_items.clear();
                } else if Id::eq_opt(&curr_conflict.right_id(), &item.right_id()) {
                    // println!("->item right id is equal to item conflict right id");
                    break;
                }
                // item right id is not matched with conflict right id
            } else if conflict_left_id.is_some()
                && items_before_origin.contains(&conflict_left_id.unwrap())
            {
                if !conflict_items.contains(&conflict_left_id.unwrap()) {
                    left.clone_from(&conflict);
                    conflict_items.clear();
                }
            } else {
                break;
            }

            conflict.clone_from(&curr_conflict.right());
        }

        if let Some(left) = &left {
            integrate_after(left, item);
            // println!("integrated after left, {}", left.id());
        } else {
            // println!("parent start: {:?}", parent.id());
            integrate_start(item, parent, start, set_start)?;
            // println!("integrated at start, {:?}", item.id());
        }

        if item.right().is_none() {
            set_end(Some(item.clone()));
        }
    }

    Ok(counter)
}

#[inline]
fn integrate_start<F>(
    item: &Type,
    parent: &Type,
    start: Option<Type>,
    set_start: F,
) -> Result<(), String>
where
    F: FnOnce(Option<Type>),
{
    if let Some(start) = start {
        start.set_left(item.clone());
        item.set_right(start);
    }
    set_start(Some(item.clone()));
    item.set_parent_id(parent.id());
    item.set_parent(parent.clone());

    Ok(())
}

#[inline]
fn integrate_after(prev: &Type, item: &Type) {
    let next = prev.right();
    let parent = prev.parent();

    prev.set_right(item.clone());

    if let Some(next) = &next {
        next.set_left(item.clone());
        item.set_right(next.clone());
    }

    item.set_left(prev.clone());
    if let Some(next) = &next {
        next.set_left(item.clone());
    }

    item.set_parent_id(parent.as_ref().map(|p| p.id()));
    item.set_parent(parent);
}

/// remove the item from the connected list
#[inline]
pub(crate) fn remove_yata(item: &Type) {
    let left = item.left();
    let right = item.right();

    if let Some(left) = &left {
        left.set_right(right.clone());
    }

    if let Some(right) = &right {
        right.set_left(left.clone());
    }

    item.set_left(None);
    item.set_right(None);
    item.set_parent(None);
}
