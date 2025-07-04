use crate::bimapid::{FieldId, FieldMap};
use crate::delete::DeleteItem;
use crate::natom::NAtom;
use crate::nlist::NList;
use crate::nmap::NMap;
use crate::nmove::NMove;
use crate::store::WeakStoreRef;
use crate::{Id, NString};
use crate::{Item, WithId};
use crate::{NText, Type};
use std::cell::RefCell;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

// InnerType auto implements common methods for inner types
pub(crate) trait InnerType: Deref<Target = Item> + DerefMut<Target = Item> {
    fn store(&self) -> &WeakStoreRef;
    fn flag(&self) -> &u8;
    fn flag_mut(&mut self) -> &mut u8;
    fn size(&self) -> usize;
    fn parent(&self) -> &Option<Type>;

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

    fn set_left_id(&mut self, left_id: Id) {
        self.left_id = Some(left_id);
    }

    fn set_right_id(&mut self, right_id: Id) {
        self.right_id = Some(right_id);
    }

    fn set_parent_id(&mut self, right_id: Id) {
        self.right_id = Some(right_id);
    }

    fn get_left_id(&mut self) -> Option<Id> {
        self.left_id.clone()
    }

    fn get_right_id(&self) -> Option<Id> {
        self.right_id.clone()
    }

    fn get_parent_id(&self) -> Option<Id> {
        self.parent_id.clone()
    }

    fn depth(&self) -> usize {
        self.parent().as_ref().map_or(0, |p| p.depth() + 1)
    }

    #[inline]
    fn delete(&mut self) {
        let store = self.store().upgrade().unwrap();
        let id = store.borrow_mut().next_id();
        let item = DeleteItem::new(id, self.id.range(1));
        store.borrow_mut().insert_delete(item);
        self.mark_deleted();
    }

    #[inline]
    fn field(&self, fields: &FieldMap) -> Option<String> {
        if let Some(field) = self.field_id {
            return fields.get_field(&field).cloned().map(|id| id.to_string());
        }

        None
    }
}
