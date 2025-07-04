use crate::bimapid::{FieldId, FieldMap};
use crate::delete::DeleteItem;
use crate::id::{Id, IdRange, WithId, WithIdRange};
use crate::inner_type::InnerType;
use crate::item::{Content, ItemData, ItemKind, ItemKindFlags};
use crate::mark::Mark;
use crate::store::WeakStoreRef;
use crate::{Item, Type};
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct AtomRef(Rc<RefCell<NAtom>>);

impl Hash for AtomRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

// Atom is a holds a fixed Content
#[derive(Debug)]
pub(crate) struct NAtom {
    pub(crate) flags: u8,        // 1
    pub(crate) item: Item,       // 16
    pub(crate) content: Content, // 7
}

impl NAtom {
    pub(crate) fn new(id: Id, content: Content, store: WeakStoreRef) -> Self {
        Self {
            flags: 0,
            item: Item::new(id),
            content,
        }
    }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        1
    }

    pub(crate) fn depth(&self) -> usize {
        panic!("depth is not implemented for NAtom");
    }

    #[inline]
    pub(crate) fn content(&self) -> Content {
        self.content.clone()
    }

    // #[inline]
    // pub(crate) fn to_json(&self) -> serde_json::Value {
    //     self.content.to_json()
    // }
}

impl Deref for NAtom {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl DerefMut for NAtom {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl Hash for NAtom {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl WithId for NAtom {
    #[inline]
    fn id(&self) -> Id {
        self.id
    }
}

impl WithIdRange for NAtom {
    #[inline]
    fn range(&self) -> IdRange {
        self.id.range(1)
    }
}

impl Serialize for NAtom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut atom = serializer.serialize_struct("Atom", 1)?;

        // self.serialize_with(&mut atom)?;

        // atom.serialize_field("content", &self.borrow().content)?;

        atom.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_a_item_data() {
        // let data = AItem::new(Id::new(1, 1))
        //     .with_kind(ItemKind::Atom)
        //     .with_left(Id::new(2, 2))
        //     .with_right(Id::new(3, 3));
        //
        // println!("size: {}", size_of::<WeakStoreRef>());
        // println!("size: {}", size_of::<InnerAtom>());
        // println!("size: {}", size_of::<AItem>());
        // println!("size: {}", size_of::<Content>());
        // assert_eq!(data.kind(), ItemKind::Atom);
        // assert_eq!(data.id(), Id::new(1, 1));
        // assert_eq!(data.left_id(), Some(Id::new(2, 2)));
        // assert_eq!(data.right_id(), Some(Id::new(3, 3)));
    }
}
