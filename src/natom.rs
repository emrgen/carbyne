use crate::bimapid::FieldId;
use crate::id::{Id, IdRange, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind, ItemKindFlags, ItemRef};
use crate::store::WeakStoreRef;
use crate::Type;
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone)]
pub struct AItemData(u128);

const M4: u128 = (1 << 4) - 1;
const M8: u128 = (1 << 8) - 1;
const M20: u128 = (1 << 20) - 1;
const M40: u128 = (1 << 40) - 1;

const U8: u32 = (1 << 8) - 1;
const U20: u32 = (1 << 20) - 1;

impl AItemData {
    fn new(id: Id) -> AItemData {
        let mut data = AItemData(0);
        data.with_id(id)
    }

    fn with_kind(mut self, kind: ItemKind) -> AItemData {
        self.0 |= ItemKindFlags::from(kind).bits() as u128;
        self
    }

    fn with_id(mut self, id: Id) -> AItemData {
        self.0 |= ((((id.client & U20) << 20) | (id.clock & U20)) as u128) << 8;
        self
    }

    fn with_left(mut self, id: Id) -> AItemData {
        self.0 |= ((((id.client & U20) << 20) | (id.clock & U20)) as u128) << 48;
        self.0 |= (1 << 5);
        self
    }

    fn with_right(mut self, id: Id) -> AItemData {
        self.0 |= ((((id.client & U20) << 20) | (id.clock & U20)) as u128) << 88;
        self.0 |= (1 << 6);
        self
    }

    fn with_field(mut self, field: FieldId) -> AItemData {
        self.0 |= ((field & U20) as u128) << 88;
        self.0 |= (1 << 4);
        self
    }

    #[inline]
    fn kind(&self) -> ItemKind {
        ItemKindFlags::from_bits((self.0 & M4) as u8)
            .unwrap()
            .into()
    }

    #[inline]
    fn flag(&self) -> u8 {
        (self.0 & M8) as u8
    }

    #[inline]
    fn id(&self) -> Id {
        AItemData::decode_id(self.0 >> 8)
    }

    #[inline]
    fn left_id(&self) -> Option<Id> {
        if self.has_left() {
            return Some(Self::decode_id(self.0 >> 48));
        }

        None
    }

    #[inline]
    fn right_id(&self) -> Option<Id> {
        if self.has_right() {
            return Some(Self::decode_id(self.0 >> 88));
        }

        None
    }

    #[inline]
    fn field(&self) -> Option<FieldId> {
        if self.has_field() {
            return Some(((self.0 >> 48) & 20) as FieldId);
        }

        None
    }

    #[inline]
    fn has_field(&self) -> bool {
        self.flag() & (1 << 4) != 0
    }

    #[inline]
    fn has_left(&self) -> bool {
        self.flag() & (1 << 5) != 0
    }

    #[inline]
    fn has_right(&self) -> bool {
        self.flag() & (1 << 6) != 0
    }

    #[inline]
    fn decode_id(from: u128) -> Id {
        Id::new((from & M20) as u32, ((from >> 20) & M20) as u32)
    }
}

impl Debug for AItemData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

// Atom is a holds a fixed Content
#[derive(Debug)]
pub struct InnerAtom {
    pub(crate) store: WeakStoreRef,
    pub(crate) data: AItemData,
    pub(crate) content: Content,
}

impl Deref for InnerAtom {
    type Target = AItemData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Clone, Debug)]
pub struct NAtom {
    inner: Rc<RefCell<InnerAtom>>,
}

impl NAtom {
    pub(crate) fn new(id: Id, content: Content, store: WeakStoreRef) -> Self {
        let data = AItemData::new(id).with_kind(ItemKind::Atom);
        Self {
            inner: Rc::new(RefCell::new(InnerAtom {
                store,
                data,
                content,
            })),
        }
    }

    #[inline]
    pub(crate) fn depth(&self) -> u32 {
        unimplemented!()
    }

    #[inline]
    pub(crate) fn size(&self) -> u32 {
        1
    }

    #[inline]
    pub(crate) fn content(&self) -> Content {
        self.borrow().content.clone()
    }

    #[inline]
    pub(crate) fn delete(&self) {
        //self.item.delete(1);
    }

    #[inline]
    pub(crate) fn item_ref(&self) -> ItemRef {
        // self.item.clone()
        unimplemented!()
    }

    #[inline]
    pub(crate) fn to_json(&self) -> serde_json::Value {
        self.borrow().content.to_json()
    }
}

impl Serialize for NAtom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut atom = serializer.serialize_struct("Atom", 1)?;

        // self.serialize_with(&mut atom)?;

        atom.serialize_field("content", &self.borrow().content)?;

        atom.end()
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
        self.borrow().id()
    }
}

impl WithIdRange for NAtom {
    #[inline]
    fn range(&self) -> IdRange {
        self.borrow().id().range(1)
    }
}

impl Deref for NAtom {
    type Target = Rc<RefCell<InnerAtom>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<ItemRef> for NAtom {
    fn from(item: ItemRef) -> Self {
        // Self { item }
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_a_item_data() {
        let data = AItemData::new(Id::new(1, 1))
            .with_kind(ItemKind::Atom)
            .with_left(Id::new(2, 2))
            .with_right(Id::new(3, 3));

        println!("size: {}", size_of::<WeakStoreRef>());
        println!("size: {}", size_of::<InnerAtom>());
        println!("size: {}", size_of::<AItemData>());
        println!("size: {}", size_of::<Content>());
        assert_eq!(data.kind(), ItemKind::Atom);
        assert_eq!(data.id(), Id::new(1, 1));
        assert_eq!(data.left_id(), Some(Id::new(2, 2)));
        assert_eq!(data.right_id(), Some(Id::new(3, 3)));
    }
}
