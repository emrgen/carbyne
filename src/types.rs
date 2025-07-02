use crate::bimapid::FieldMap;
use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::delete::DeleteItem;
use crate::doc::{Doc, DocMeta};
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, Split, WithId, WithIdRange, WithTarget};
use crate::inner_type::InnerFlag;
use crate::item::{Content, ItemData, ItemKey, ItemKind, ItemRef, WithIndex};
use crate::mark::Mark;
use crate::natom::{AItemData, NAtom};
use crate::nlist::NList;
use crate::nmap::NMap;
use crate::nmark::NMark;
use crate::nmove::NMove;
use crate::nstring::NString;
use crate::ntext::NText;
use crate::store::{StoreRef, WeakStoreRef};
use crate::{print_yaml, Client};
use fake::Opt;
use fractional_index::FractionalIndex;
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;
use std::ops::Range;

/// Type is a wrapper around the different item types in the store.
#[derive(Debug, Clone, Default, Hash)]
pub enum Type {
    List(NList),     // container
    Map(NMap),       // container
    Text(NText),     // container
    String(NString), // elementary
    Atom(NAtom),     // elementary
    Move(NMove),     // elementary
    Mark(NMark),     // elementary
    #[default]
    Identity, // used for empty items
}

impl Type {
    pub(crate) fn replace(&self, items: (Type, Type)) {
        if let Some(parent) = self.get_parent() {
            // parent.replace_child(self, items);
        }
    }
}

impl Type {
    pub(crate) fn slice(&self, start: u32, end: u32) -> Vec<Type> {
        match self {
            // Type::Text(n) => n.slice(start, end),
            _ => panic!("slice: not implemented"),
        }
    }
}

impl Type {
    #[inline]
    pub(crate) fn as_list(&self) -> Option<NList> {
        match self {
            Type::List(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn as_map(&self) -> Option<NMap> {
        match self {
            Type::Map(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn as_text(&self) -> Option<NText> {
        match self {
            Type::Text(n) => Some(n.clone()),
            _ => None,
        }
    }
    #[inline]
    pub(crate) fn as_string(&self) -> Option<NString> {
        match self {
            Type::String(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn as_atom(&self) -> Option<NAtom> {
        match self {
            Type::Atom(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn as_move(&self) -> Option<NMove> {
        match self {
            Type::Move(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn as_mark(&self) -> Option<NMark> {
        match self {
            Type::Mark(n) => Some(n.clone()),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn as_doc(&self) -> Option<Doc> {
        match self {
            // Type::Doc(n) => Some(n.clone()),
            _ => None,
        }
    }

    pub(crate) fn data(&self) -> AItemData {
        match self {
            // Type::List(n) => n.data(),
            // Type::Map(n) => n.data(),
            // Type::Text(n) => n.data(),
            // Type::String(n) => n.data(),
            // Type::Atom(n) => n.data(),
            // Type::Move(n) => n.data(),
            // Type::Mark(n) => n.data(),
            _ => panic!("data: not implemented"),
        }
    }

    // #[inline]
    // pub(crate) fn store(&self) -> WeakStoreRef {
    //     match self {
    //         Type::List(n) => n.get_left_id(),
    //         Type::Map(n) => n.get_left_id(),
    //         Type::Text(n) => n.get_left_id(),
    //         Type::String(n) => n.get_left_id(),
    //         Type::Move(n) => n.get_left_id(),
    //         _ => panic!("set_left_id: not implemented"),
    //     }
    // }

    #[inline]
    pub(crate) fn mark_deleted(&self) {
        match self {
            Type::List(n) => n.mark_deleted(),
            Type::Map(n) => n.mark_deleted(),
            Type::Text(n) => n.mark_deleted(),
            Type::String(n) => n.mark_deleted(),
            Type::Atom(n) => n.mark_deleted(),
            Type::Move(n) => n.mark_deleted(),
            Type::String(n) => n.mark_deleted(),
            _ => panic!("mark_deleted: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn unmark_deleted(&self) {
        match self {
            Type::List(n) => n.unmark_deleted(),
            Type::Map(n) => n.unmark_deleted(),
            Type::Text(n) => n.unmark_deleted(),
            Type::String(n) => n.unmark_deleted(),
            Type::Atom(n) => n.unmark_deleted(),
            Type::Move(n) => n.unmark_deleted(),
            Type::String(n) => n.unmark_deleted(),
            _ => panic!("unmark_deleted: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn mark_moved(&self) {
        match self {
            Type::List(n) => n.mark_moved(),
            Type::Map(n) => n.mark_moved(),
            Type::Text(n) => n.mark_moved(),
            Type::String(n) => n.mark_moved(),
            Type::Atom(n) => n.mark_moved(),
            Type::Move(n) => n.mark_moved(),
            Type::String(n) => n.mark_moved(),
            _ => panic!("mark_moved: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn unmark_moved(&self) {
        match self {
            Type::List(n) => n.unmark_moved(),
            Type::Map(n) => n.unmark_moved(),
            Type::Text(n) => n.unmark_moved(),
            Type::String(n) => n.unmark_moved(),
            Type::Atom(n) => n.unmark_moved(),
            Type::Move(n) => n.unmark_moved(),
            Type::String(n) => n.unmark_moved(),
            _ => panic!("unmark_moved: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn mark_active(&self) {
        match self {
            Type::List(n) => n.mark_active(),
            Type::Map(n) => n.mark_active(),
            Type::Text(n) => n.mark_active(),
            Type::String(n) => n.mark_active(),
            Type::Atom(n) => n.mark_active(),
            Type::Move(n) => n.mark_active(),
            Type::String(n) => n.mark_active(),
            _ => panic!("mark_active: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn unmark_active(&self) {
        match self {
            Type::List(n) => n.unmark_active(),
            Type::Map(n) => n.unmark_active(),
            Type::Text(n) => n.unmark_active(),
            Type::String(n) => n.unmark_active(),
            Type::Atom(n) => n.unmark_active(),
            Type::Move(n) => n.unmark_active(),
            Type::String(n) => n.unmark_active(),
            _ => panic!("mark_inactive: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn depth(&self) -> usize {
        match self {
            Type::List(n) => n.depth(),
            Type::Map(n) => n.depth(),
            Type::Text(n) => n.depth(),
            Type::Move(n) => n.depth(),
            _ => panic!("depth: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn get_left_id(&self) -> Option<Id> {
        match self {
            Type::List(n) => n.get_left_id(),
            Type::Map(n) => n.get_left_id(),
            Type::Text(n) => n.get_left_id(),
            Type::String(n) => n.get_left_id(),
            Type::Move(n) => n.get_left_id(),
            _ => panic!("set_left_id: not implemented"),
        }
    }

    pub(crate) fn get_right_id(&self) -> Option<Id> {
        match self {
            Type::List(n) => n.get_right_id(),
            Type::Map(n) => n.get_right_id(),
            Type::Text(n) => n.get_right_id(),
            Type::String(n) => n.get_right_id(),
            Type::Move(n) => n.get_right_id(),
            _ => panic!("get_right_id: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn get_parent_id(&self) -> Option<Id> {
        match self {
            Type::List(n) => n.get_parent().map(|p| p.id()),
            Type::Map(n) => n.get_parent().map(|p| p.id()),
            Type::Text(n) => n.get_parent().map(|p| p.id()),
            Type::String(n) => n.get_parent().map(|p| p.id()),
            Type::Move(n) => n.get_parent().map(|p| p.id()),
            _ => panic!("get_parent_id: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn get_parent(&self) -> Option<Type> {
        match self {
            Type::List(n) => n.get_parent(),
            Type::Map(n) => n.get_parent(),
            Type::Text(n) => n.get_parent(),
            Type::String(n) => n.get_parent(),
            Type::Move(n) => n.get_parent(),
            _ => panic!("parent_id: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn set_parent(&self, parent: impl Into<Option<Type>>) {
        let parent = parent.into();
        match self {
            Type::List(n) => n.set_parent(parent),
            Type::Map(n) => n.set_parent(parent),
            Type::Text(n) => n.set_parent(parent),
            Type::String(n) => n.set_parent(parent),
            Type::Move(n) => n.set_parent(parent),
            _ => panic!("set_left_id: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn set_left_id(&self, left_id: impl Into<Id>) {
        let left_id = left_id.into();
        match self {
            Type::List(n) => n.set_left_id(left_id),
            Type::Map(n) => n.set_left_id(left_id),
            Type::Text(n) => n.set_left_id(left_id),
            Type::String(n) => n.set_left_id(left_id),
            Type::Atom(n) => n.set_left_id(left_id),
            Type::Move(n) => n.set_left_id(left_id),
            // Type::Mark(n) => n.set_right_id(right_id),
            _ => panic!("set_left_id: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn set_right_id(&self, right_id: impl Into<Id>) {
        let right_id = right_id.into();
        match self {
            Type::List(n) => n.set_right_id(right_id),
            Type::Map(n) => n.set_right_id(right_id),
            Type::Text(n) => n.set_right_id(right_id),
            Type::String(n) => n.set_right_id(right_id),
            Type::Atom(n) => n.set_right_id(right_id),
            Type::Move(n) => n.set_right_id(right_id),
            _ => panic!("set_right_id: not implemented"),
        }
    }

    // insert after skips the list index lookup and directly inserts the item after the current item
    pub fn insert_after(&self, item: impl Into<Type>) {
        // let item = item.into();
        //
        // let parent = self.parent();
        // let next = self.right();
        //
        // item.set_parent_id(parent.as_ref().map(|p| p.id()));
        // item.set_left_id(Some(self.id()));
        // item.set_right_id(next.as_ref().map(|n| n.id()));
        //
        // // item.set_parent(parent.clone());
        // item.set_left(self.clone());
        // item.set_right(next.clone());
        //
        // self.set_right(item.clone());
        //
        // if let Some(next) = next {
        //     next.set_left(item.clone());
        // } else if let Some(ref parent) = parent {
        //     parent.set_end(item.clone());
        // }
    }

    // insert before skips the list index lookup and directly inserts the item before the current item
    pub fn insert_before(&self, item: Type) {
        // let parent = self.parent();
        // let prev = self.left();
        //
        // item.set_parent_id(parent.as_ref().map(|p| p.id()));
        // item.set_left_id(prev.as_ref().map(|p| p.id()));
        // item.set_right_id(Some(self.id()));
        //
        // item.set_parent(parent.clone());
        // item.set_left(prev.clone());
        // item.set_right(self.clone());
        //
        // self.set_left(item.clone());
        //
        // if let Some(prev) = prev {
        //     prev.set_right(item.clone());
        // } else if let Some(ref parent) = parent {
        //     parent.set_start(item.clone());
        // }
        //
        // parent.unwrap().on_insert(&item);
    }

    #[inline]
    pub(crate) fn is_moved(&self) -> bool {
        match self {
            Type::List(n) => n.is_moved(),
            Type::Map(n) => n.is_moved(),
            Type::Text(n) => n.is_moved(),
            Type::Atom(n) => n.is_moved(),
            Type::Move(n) => n.is_moved(),
            _ => panic!("is_moved: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn is_deleted(&self) -> bool {
        match self {
            Type::List(n) => n.is_deleted(),
            Type::Map(n) => n.is_deleted(),
            Type::Text(n) => n.is_deleted(),
            Type::Atom(n) => n.is_deleted(),
            Type::Move(n) => n.is_deleted(),
            _ => panic!("is_deleted: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        match self {
            Type::List(n) => n.is_active(),
            Type::Map(n) => n.is_active(),
            Type::Text(n) => n.is_active(),
            Type::Atom(n) => n.is_active(),
            Type::Move(n) => n.is_active(),
            _ => panic!("is_active: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn is_visible(&self) -> bool {
        match self {
            Type::List(n) => n.borrow().is_visible(),
            Type::Map(n) => n.borrow().is_visible(),
            Type::Text(n) => n.borrow().is_visible(),
            Type::Atom(n) => n.borrow().is_visible(),
            Type::Move(n) => n.borrow().is_visible(),
            _ => panic!("is_active: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn index_of(&self, target: &Type) -> Option<usize> {
        match self {
            Type::List(list) => list.index_of(target),
            _ => None,
        }
    }

    #[inline]
    pub fn kind(&self) -> ItemKind {
        match self {
            Type::List(n) => ItemKind::List,
            Type::Map(n) => ItemKind::Map,
            Type::Text(n) => ItemKind::Text,
            Type::String(n) => ItemKind::String,
            Type::Atom(n) => ItemKind::Atom,
            Type::Move(n) => ItemKind::Move,
            // Type::Move(n) => ItemKind::FlatMove,
            Type::Mark(n) => ItemKind::Mark,
            _ => panic!("size: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn field(&self, fields: &FieldMap) -> Option<String> {
        match self {
            // Type::List(n) => n.field(),
            // Type::Map(n) => n.field(),
            // Type::Text(n) => n.field(),
            Type::Atom(n) => n.field(fields),
            _ => panic!("size: not implemented"),
        }
    }

    // disconnect from the document tree
    pub(crate) fn disconnect(&self) {
        // print_yaml(&self);
        match self.get_parent() {
            // Some(Type::List(n)) => n.remove_child(self),
            // Some(Type::Map(n)) => n.remove_child(self),
            // Type::Text(n) => n.add_mark(mark),
            // Some(parent) => panic!("disconnect is not implemented for {}", parent.kind()),
            _ => panic!("disconnect called for an orphan item"),
        }
    }

    pub(crate) fn add_mark(&self, mark: Mark, range: Range<usize>) {
        match self {
            // Type::Text(n) => n.add_mark(mark, range),
            _ => panic!("add_mark: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn remove_mark(&self, mark: Mark) {
        // let id = self.store().upgrade().unwrap().borrow_mut().next_id();
        // // let marks = self.item_ref().borrow().get_marks();
        // let item = DeleteItem::new(id, self.range());
    }

    #[inline]
    pub fn get_target(&self) -> Option<Type> {
        match self {
            Type::Move(n) => n.get_target(),
            _ => panic!("target: not implemented"),
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        match self {
            Type::List(n) => n.size(),
            Type::Map(n) => n.size(),
            Type::Text(n) => n.size(),
            Type::String(n) => n.size(),
            Type::Atom(n) => n.size(),
            Type::Move(n) => n.size(),
            Type::Mark(n) => n.size(),
            _ => panic!("size: not implemented"),
        }
    }

    // pub fn content(&self) -> Content {
    //     match self {
    //         Type::String(n) => n.content(),
    //         Type::Atom(n) => n.content(),
    //         Type::Text(n) => n.content(),
    //         Type::Move(n) => n.content(),
    //         // Type::Mark(n) => n.content(),
    //         Type::List(n) => n.content(),
    //         Type::Map(n) => n.content(),
    //         _ => {
    //             panic!("content: not implemented for {:?}", self.kind())
    //         }
    //     }
    // }

    /// move the item to the given parent at the given offset
    pub fn move_to(&self, parent: impl Into<Type>, offset: usize) {
        let parent = parent.into();
        match parent {
            Type::List(n) => n.move_to(offset, self),
            _ => panic!(
                "move: not implemented for {:?} to parent type: {:?}",
                self.kind(),
                parent.kind()
            ),
        }
    }

    /// move the item after the given item
    pub fn move_after(&self, before: &Type) {
        if let Some(parent) = before.get_parent() {
            match parent {
                Type::List(n) => n.move_after(before, self),
                _ => panic!(
                    "move: not implemented for {:?} to parent type: {:?}",
                    self.kind(),
                    parent.kind()
                ),
            }
        }
    }

    /// move the item before the given item
    pub fn move_before(&self, after: &Type) {
        if let Some(parent) = after.get_parent() {
            match parent {
                Type::List(n) => n.move_before(after, self),
                _ => panic!(
                    "move: not implemented for {:?} to parent type: {:?}",
                    self.kind(),
                    parent.kind()
                ),
            }
        }
    }

    #[inline]
    pub fn append(&self, item: impl Into<Type>) {
        match self {
            Type::List(n) => n.append(item),
            Type::Text(n) => n.append(item),
            _ => panic!("append: not implemented"),
        }
    }

    #[inline]
    pub fn prepend(&self, item: impl Into<Type>) {
        match self {
            Type::List(n) => n.prepend(item),
            Type::Text(n) => n.prepend(item),
            _ => panic!("prepend: not implemented"),
        }
    }

    #[inline]
    pub fn insert(&self, offset: usize, item: impl Into<Type>) {
        match self {
            Type::List(n) => n.insert(offset, item),
            Type::Text(n) => {
                let item = item.into();
                assert!(item.is_string());
                n.insert(offset, item)
            }
            _ => panic!("insert: not implemented"),
        }
    }

    #[inline]
    pub fn set(&self, key: impl Into<ItemKey>, item: impl Into<Type>) {
        let key = key.into();
        match self {
            Type::Map(n) => n.set(key.as_string(), item.into()),
            _ => panic!("set: not implemented"),
        }
    }

    #[inline]
    pub fn get(&self, key: impl Into<ItemKey>) -> Option<Type> {
        match self {
            Type::Map(n) => n.get(key.into()),
            Type::List(n) => n.get(key.into()),
            _ => panic!("get: not implemented"),
        }
    }

    #[inline]
    pub fn remove(&self, key: ItemKey) {
        match self {
            Type::Map(n) => n.remove(key),
            _ => panic!("remove: not implemented"),
        }
    }

    #[inline]
    pub fn delete(&self) {
        match self {
            Type::List(n) => n.delete(),
            Type::Map(n) => n.delete(),
            Type::Text(n) => n.delete(),
            // Type::Atom(n) => n.delete(),
            _ => panic!("clear: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn clear(&self) {
        match self {
            Type::List(n) => n.clear(),
            Type::Map(n) => n.clear(),
            Type::Text(n) => n.clear(),
            _ => panic!("clear: not implemented"),
        }
    }

    pub(crate) fn text_content(&self) -> String {
        match self {
            Type::Text(n) => n.text_content(),
            // Type::Atom(n) => n.text_content(),
            _ => {
                panic!("text_content: not implemented for {:?}", self.kind())
            }
        }
    }

    pub(crate) fn eq_opt(a: &Option<Type>, b: &Option<Type>) -> bool {
        let a = a.as_ref().map(|a| a.id());
        let b = b.as_ref().map(|b| b.id());
        Id::eq_opt(&a, &b)
    }

    #[inline]
    pub(crate) fn split(&self, offset: usize) -> (Type, Type) {
        match self {
            // Type::String(n) => n.split(offset).unwrap(),
            // Type::Mark(n) => n.split(offset).unwrap(),
            _ => panic!("split: not implemented for {:?}", self.kind()),
        }
    }

    #[inline]
    pub(crate) fn to_json(&self) -> Value {
        match self {
            Type::List(n) => n.to_json(),
            Type::Map(n) => n.to_json(),
            Type::Text(n) => n.to_json(),
            Type::String(n) => n.to_json(),
            Type::Atom(n) => n.to_json(),
            Type::Move(n) => n.to_json(),
            Type::Mark(n) => n.to_json(),
            // Type::Doc(n) => n.to_json(),
            Type::Identity => panic!("to_json: not implemented for identity"),
        }
    }

    fn is_string(&self) -> bool {
        match self {
            Type::String(_) => true,
            _ => false,
        }
    }

    fn is_mark(&self) -> bool {
        match self {
            Type::Mark(_) => true,
            _ => false,
        }
    }

    fn is_list(&self) -> bool {
        match self {
            Type::List(_) => true,
            _ => false,
        }
    }

    fn is_map(&self) -> bool {
        match self {
            Type::Map(_) => true,
            _ => false,
        }
    }

    fn is_text(&self) -> bool {
        match self {
            Type::Text(_) => true,
            _ => false,
        }
    }

    fn is_atom(&self) -> bool {
        match self {
            Type::Atom(_) => true,
            _ => false,
        }
    }

    fn is_move(&self) -> bool {
        match self {
            Type::Move(_) => true,
            _ => false,
        }
    }

    pub(crate) fn on_insert(&self, child: &Type) {
        match self {
            // Type::List(n) => n.on_insert(child),
            // Type::Text(n) => n.on_insert(child),
            // Type::Map(n) => {}
            _ => panic!("on_insert: not implemented for {:?}", self.kind()),
        }
    }

    pub(crate) fn on_delete(&self, child: &Type) {}

    pub(crate) fn on_undelete(&self, child: &Type) {}

    pub(crate) fn on_move(&self, child: &Type) {}

    pub(crate) fn on_unmove(&self, child: &Type) {}
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            Type::List(n) => n.serialize(serializer),
            Type::Map(n) => n.serialize(serializer),
            Type::Text(n) => n.serialize(serializer),
            Type::String(n) => n.serialize(serializer),
            Type::Atom(n) => n.serialize(serializer),
            Type::Mark(n) => n.serialize(serializer),
            Type::Move(n) => n.serialize(serializer),
            _ => panic!("Type: serialize: not implemented for {:?}", self),
        }
    }
}

impl Encode for Type {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        match &self {
            Type::List(n) => n.encode(e, ctx),
            // Type::Map(n) => n.id(),
            // Type::Text(n) => n.id(),
            // Type::String(n) => n.id(),
            // Type::Atom(n) => n.id(),
            // Type::Move(n) => n.id(),
            // Type::Mark(n) => n.id(),
            Type::Identity => panic!("id: not implemented for identity"),
            _ => {}
        }
        // e.item(ctx, &self.item_ref().borrow().data.clone());
    }
}

impl Decode for Type {
    #[inline]
    fn decode<T: Decoder>(_d: &mut T, _ctx: &DecodeContext) -> Result<Self, String> {
        Err("Type::decode: not implemented".to_string())
    }
}

impl WithId for Type {
    #[inline]
    fn id(&self) -> Id {
        match &self {
            Type::List(n) => n.id(),
            Type::Map(n) => n.id(),
            Type::Text(n) => n.id(),
            Type::String(n) => n.id(),
            Type::Atom(n) => n.id(),
            Type::Move(n) => n.id(),
            Type::Mark(n) => n.id(),
            Type::Identity => panic!("id: not implemented for identity"),
        }
    }
}

impl From<Type> for Option<Id> {
    #[inline]
    fn from(value: Type) -> Self {
        Some(value.id())
    }
}

impl From<&Type> for Option<Id> {
    #[inline]
    fn from(value: &Type) -> Self {
        Some(value.id())
    }
}

impl WithIdRange for Type {
    fn range(&self) -> IdRange {
        match &self {
            Type::List(n) => n.range(),
            Type::Map(n) => n.range(),
            Type::Text(n) => n.range(),
            Type::String(n) => n.range(),
            Type::Atom(n) => n.range(),
            Type::Move(n) => n.range(),
            Type::Mark(n) => n.range(),
            // Type::Doc(n) => n.root.range(),
            Type::Identity => panic!("range: not implemented for identity"),
        }
    }
}

impl From<NList> for Type {
    fn from(n: NList) -> Self {
        Self::List(n)
    }
}

impl From<NMap> for Type {
    fn from(n: NMap) -> Self {
        Self::Map(n)
    }
}

impl From<NText> for Type {
    fn from(n: NText) -> Self {
        Self::Text(n)
    }
}

impl From<NString> for Type {
    fn from(n: NString) -> Self {
        Self::String(n)
    }
}

impl From<NAtom> for Type {
    fn from(n: NAtom) -> Self {
        Self::Atom(n)
    }
}

impl From<NMove> for Type {
    fn from(n: NMove) -> Self {
        Self::Move(n)
    }
}

impl From<NMark> for Type {
    fn from(n: NMark) -> Self {
        Self::Mark(n)
    }
}

impl<T: Into<Type> + Clone> From<&T> for Type {
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id().cmp(&other.id())
    }
}
impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Some(self.cmp(other))
        Option::from(Ordering::Equal)
    }
}

impl PartialEq<Self> for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for Type {}
