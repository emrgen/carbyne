use crate::bimapid::{FieldId, FieldMap};
use crate::delete::DeleteItem;
use crate::natom::{AItem, InnerAtom, NAtom};
use crate::nlist::{InnerList, NList};
use crate::nmap::{InnerMap, NMap};
use crate::nmove::{InnerMove, NMove};
use crate::nstring::InnerString;
use crate::ntext::InnerText;
use crate::store::WeakStoreRef;
use crate::WithId;
use crate::{Id, NString};
use crate::{NText, Type};
use std::cell::RefCell;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::rc::Rc;

pub(crate) trait InnerFlag {
    fn flag(&self) -> &u8;
    fn flag_mut(&mut self) -> &mut u8;

    fn is_visible(&self) -> bool {
        !(self.is_deleted() || self.is_moved() || !self.is_active())
    }

    fn is_deleted(&self) -> bool {
        (self.flag() | 0x01) == 0
    }

    fn is_moved(&self) -> bool {
        (self.flag() | 0x02) == 0
    }

    fn is_active(&self) -> bool {
        (self.flag() | 0x03) == 0
    }

    #[inline]
    fn mark_deleted(&mut self) {
        *self.flag_mut() |= 0x01;
    }

    #[inline]
    fn unmark_deleted(&mut self) {
        *self.flag_mut() &= !0x01;
    }

    fn mark_moved(&mut self) {
        *self.flag_mut() |= 0x02;
    }

    fn unmark_moved(&mut self) {
        *self.flag_mut() &= !0x02;
    }

    fn mark_active(&mut self) {
        *self.flag_mut() |= 0x03;
    }

    fn unmark_active(&mut self) {
        *self.flag_mut() &= !0x03;
    }
}

macro_rules! impl_inner_flag {
    ($t:ty) => {
        impl InnerFlag for $t {
            #[inline]
            fn flag(&self) -> &u8 {
                &self.flags
            }
            #[inline]
            fn flag_mut(&mut self) -> &mut u8 {
                &mut self.flags
            }
        }
    };
}

macro_rules! impl_inner_flag_many {
    ($($t:ty),*) => {
        $(impl_inner_flag!($t);)*
    };
}

impl_inner_flag_many!(
    InnerList,
    InnerMap,
    InnerText,
    InnerString,
    InnerAtom,
    InnerMove
);

pub(crate) trait InnerType: InnerFlag {
    fn store(&self) -> &WeakStoreRef;
    fn data(&self) -> &AItem;
    fn data_mut(&mut self) -> &mut AItem;

    #[inline]
    fn id(&self) -> Id {
        self.data().id()
    }

    #[inline]
    fn delete(&mut self) {
        let store = self.store().upgrade().unwrap();
        let id = store.borrow_mut().next_id();
        let item = DeleteItem::new(id, self.id().range(1));
        store.borrow_mut().insert_delete(item);
        self.mark_deleted();
    }

    fn field(&self, fields: &FieldMap) -> Option<String> {
        if let Some(field) = self.data().field_id() {
            return fields.get_field(&field).cloned().map(|id| id.to_string());
        }

        None
    }
}

macro_rules! impl_inner_type_inner {
    ($t:ty) => {
        impl InnerType for $t {
            fn store(&self) -> &WeakStoreRef {
                &self.store
            }
            fn data(&self) -> &AItem {
                &self.data
            }
            fn data_mut(&mut self) -> &mut AItem {
                &mut self.data
            }
        }

        impl Deref for $t {
            type Target = AItem;

            fn deref(&self) -> &Self::Target {
                &self.data
            }
        }
    };
}

macro_rules! impl_inner_type_inner_many {
    ($($t:ty),*) => {
        $(impl_inner_type_inner!($t);)*
    };
}

impl_inner_type_inner_many!(InnerList, InnerMap, InnerText);

macro_rules! impl_inner_data_type {
    ($t:ty) => {
        impl $t {
            #[inline]
            pub(crate) fn set_field_id(&self, field_id: FieldId) {
                self.borrow_mut().data.set_field(field_id);
            }

            #[inline]
            pub(crate) fn get_field_id(&self) -> Option<FieldId> {
                self.borrow_mut().data.field_id().clone()
            }

            #[inline]
            pub(crate) fn set_left_id(&self, left_id: Id) {
                self.borrow_mut().data.set_left(left_id);
            }

            #[inline]
            pub(crate) fn get_left_id(&self) -> Option<Id> {
                self.borrow_mut().data.left_id().clone()
            }

            #[inline]
            pub(crate) fn set_right_id(&self, right_id: Id) {
                self.borrow_mut().data.set_right(right_id);
            }

            #[inline]
            pub(crate) fn get_right_id(&self) -> Option<Id> {
                self.borrow_mut().data.right_id().clone()
            }

            #[inline]
            pub(crate) fn set_parent_id(&self, parent_id: Id) {
                self.borrow_mut().data.set_parent(parent_id);
            }

            #[inline]
            pub(crate) fn get_parent_id(&self) -> Option<Id> {
                self.borrow_mut().data.parent_id().clone()
            }

            pub(crate) fn is_deleted(&self) -> bool {
                self.borrow().is_deleted()
            }

            pub(crate) fn is_moved(&self) -> bool {
                self.borrow().is_moved()
            }

            pub(crate) fn is_active(&self) -> bool {
                self.borrow().is_active()
            }

            pub(crate) fn mark_moved(&self) {
                self.borrow_mut().mark_moved();
            }

            pub(crate) fn unmark_moved(&self) {
                self.borrow_mut().unmark_moved();
            }

            pub(crate) fn mark_deleted(&self) {
                self.borrow_mut().mark_deleted();
            }

            pub(crate) fn unmark_deleted(&self) {
                self.borrow_mut().unmark_deleted();
            }

            pub(crate) fn mark_active(&self) {
                self.borrow_mut().mark_active();
            }

            pub(crate) fn unmark_active(&self) {
                self.borrow_mut().unmark_active();
            }
        }
    };
}

macro_rules! impl_inner_type_data_many {
    ($($t:ty),*) => {
        $(impl_inner_data_type!($t);)*
    };
}

impl_inner_type_data_many!(NList, NMap, NText, NAtom, NMove, NString);

macro_rules! impl_inner_type {
    ($t:ty) => {
        impl $t {
            pub(crate) fn depth(&self) -> usize {
                self.borrow().parent.as_ref().map_or(0, |t| t.depth() + 1)
            }

            #[inline]
            pub(crate) fn field(&self, fields: &FieldMap) -> Option<String> {
                self.borrow().field(fields)
            }

            #[inline]
            pub(crate) fn set_parent(&self, parent: Option<Type>) {
                self.borrow_mut().parent = parent;
            }

            #[inline]
            pub(crate) fn get_parent(&self) -> Option<Type> {
                self.borrow_mut().parent.clone()
            }

            #[inline]
            pub(crate) fn delete(&self) {
                self.borrow_mut().delete();
            }

            pub(crate) fn disconnect(&self) {}
        }

        impl WithId for $t {
            #[inline]
            fn id(&self) -> Id {
                self.borrow().id()
            }
        }

        impl Hash for $t {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.id().hash(state);
            }
        }
    };
}

macro_rules! impl_inner_type_many {
    ($($t:ty),*) => {
        $(impl_inner_type!($t);)*
    };
}

impl_inner_type_many!(NList, NMap, NText);
