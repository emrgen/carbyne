use crate::bimapid::{ClientMap, FieldId, FieldMap};
use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::delete::DeleteItem;
use crate::doc::DocId;
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, Split, WithId, WithIdRange, WithTarget};
use crate::item::Any::U32;
use crate::mark::MarkContent;
use crate::nmark::NMark;
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::{print_yaml, Client, NString};
use bitflags::bitflags;
use fractional_index::FractionalIndex;
use hashbrown::HashMap;
use indexmap::IndexMap;
use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::cell::RefCell;
use std::cmp::{Ordering, PartialEq};
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Timestamp;

type ItemRefInner = Rc<RefCell<Item>>;

#[derive(Debug, Clone, Default)]
pub struct ItemRef {
    pub(crate) store: WeakStoreRef,
    pub(crate) item: ItemRefInner,
}

impl ItemRef {
    #[inline]
    pub(crate) fn set_content(&self, content: Content) {
        self.borrow_mut().content = content;
    }

    /// Get the item depth in the document tree.
    /// as most of the nodes in a document is at shallow level this function should be very fast
    #[inline]
    pub(crate) fn depth(&self) -> u32 {
        if let Some(parent) = &self.item.borrow().parent {
            parent.depth() + 1
        } else {
            0
        }
    }

    #[inline]
    pub(crate) fn size(&self) -> u32 {
        self.item.borrow().size()
    }

    #[inline]
    pub(crate) fn text_content(&self) -> String {
        match self.borrow().content {
            Content::String(ref s) => s.clone(),
            _ => {
                panic!("NString has invalid content")
            }
        }
    }
}

pub(crate) trait WithIndex {
    fn index(&self) -> FractionalIndex;
}

impl WithIndex for ItemRef {
    #[inline]
    fn index(&self) -> FractionalIndex {
        self.item.borrow().index.clone()
    }
}

impl ItemRef {
    pub(crate) fn new(item: ItemRefInner, store: WeakStoreRef) -> Self {
        Self { item, store }
    }

    #[inline]
    pub(crate) fn kind(&self) -> ItemKind {
        self.item.borrow().kind.clone()
    }

    #[inline]
    pub(crate) fn field(&self) -> Option<String> {
        self.item.borrow().field(&self.store)
    }

    // #[inline]
    // pub(crate) fn add_mark(&self, mark: NMark) {
    //     self.borrow_mut().add_mark(mark);
    // }

    pub(crate) fn append(&self, value: impl Into<Type>) {
        let end = self.borrow().end.clone();
        let item = value.into();

        if let Some(ref end) = end {
            item.item_ref().borrow_mut().left = Some(end.clone());
            item.item_ref().borrow_mut().data.left_id = Some(end.end_id());

            end.item_ref().borrow_mut().right = Some(item.clone());
            self.borrow_mut().end = Some(item.clone());
        } else {
            self.borrow_mut().start = Some(item.clone());
            self.borrow_mut().end = Some(item.clone());
        }

        item.item_ref().borrow_mut().data.parent_id = Some(self.id());
        // item.item_ref().borrow_mut().parent = Some(Type::from(self.clone()));

        // TODO: if item and prev are adjacent string items, merge them
    }

    pub(crate) fn prepend(&self, value: impl Into<Type>) {
        let item = value.into();
        let start = self.borrow().start.clone();
        if let Some(ref start) = start {
            start.item_ref().borrow_mut().left = Some(item.clone());
            item.item_ref().borrow_mut().right = Some(start.clone());
            self.borrow_mut().start = Some(item.clone());
            item.item_ref().borrow_mut().data.right_id = Some(start.id());
        } else {
            self.borrow_mut().start = Some(item.clone());
            self.borrow_mut().end = Some(item.clone());
        }

        item.item_ref().borrow_mut().data.parent_id = Some(self.id());
        item.item_ref().borrow_mut().parent = Some(self.into());
    }

    #[inline]
    pub(crate) fn left_origin(&self) -> Option<Type> {
        self.borrow().left_origin(self.store.clone())
    }

    #[inline]
    pub(crate) fn mark_moved(&self) {
        self.borrow_mut().mark_moved();
    }

    #[inline]
    pub(crate) fn unmark_moved(&self) {
        self.borrow_mut().unmark_moved();
    }

    #[inline]
    pub(crate) fn mark_inactive(&self) {
        self.borrow_mut().mark_inactive();
    }

    #[inline]
    pub(crate) fn mark_active(&self) {
        self.borrow_mut().mark_active();
    }

    #[inline]
    pub(crate) fn is_inactive(&self) -> bool {
        self.borrow().is_inactive()
    }

    #[inline]
    pub(crate) fn is_moved(&self) -> bool {
        self.borrow().is_moved()
    }

    #[inline]
    pub(crate) fn is_deleted(&self) -> bool {
        self.borrow().is_deleted()
    }

    pub(crate) fn delete(&self, size: u32) {
        let store = self.store.upgrade().unwrap();
        let id = store.borrow_mut().next_id();
        let item = DeleteItem::new(id, self.id().range(size));
        store.borrow_mut().insert_delete(item);
        self.borrow_mut().make_deleted();
    }
}

impl ItemRef {
    pub(crate) fn serialize_with<S>(&self, s: &mut S) -> Result<(), S::Error>
    where
        S: serde::ser::SerializeStruct,
    {
        s.serialize_field("id", &self.id().to_string())?;
        s.serialize_field("kind", &self.kind().to_string())?;
        let data = &self.borrow().data;

        if let Some(parent) = &data.parent_id {
            s.serialize_field("parent_id", &parent.id().to_string())?;
        }

        if let Some(left) = &data.left_id {
            s.serialize_field("left_id", &left.id().to_string())?;
        }

        if let Some(right) = &data.right_id {
            s.serialize_field("right_id", &right.id().to_string())?;
        }

        // let marks_map = self.borrow().get_marks();
        // let mut map = serde_json::Map::new();
        // for mark in marks_map.iter() {
        //     if let Content::Mark(mark) = mark.content() {
        //         let (k, v) = mark.get_key_value();
        //         map.insert(k, v);
        //     }
        // }
        // if !map.is_empty() {
        //     let marks = serde_json::to_value(map).unwrap_or_default();
        //     s.serialize_field("marks", &marks)?;
        // }

        Ok(())
    }
}

impl Linked for ItemRef {
    #[inline]
    fn left(&self) -> Option<ItemRef> {
        self.borrow().left.as_ref().map(|l| l.item_ref().clone())
    }

    #[inline]
    fn right(&self) -> Option<ItemRef> {
        self.borrow().right.as_ref().map(|r| r.item_ref().clone())
    }

    #[inline]
    fn is_visible(&self) -> bool {
        !self.borrow().is_deleted() && !self.borrow().is_moved()
    }

    fn disconnect(&mut self) {
        if let Some(left) = self.left() {
            left.borrow_mut().right = self.right().map(|r| r.into());
        }

        if let Some(right) = self.right() {
            right.borrow_mut().left = self.left().map(|r| r.into());
        }
    }
}

pub(crate) trait StartEnd
where
    Self: Sized,
{
    fn start(&self) -> Option<Self>;
    fn end(&self) -> Option<Self>;
}

pub(crate) trait Linked: StartEnd
where
    Self: Sized,
{
    fn left(&self) -> Option<Self>;
    fn right(&self) -> Option<Self>;

    fn is_visible(&self) -> bool;
    fn disconnect(&mut self) {}
}

impl StartEnd for ItemRef {
    #[inline]
    fn start(&self) -> Option<Self> {
        self.borrow().start.as_ref().map(|s| s.item_ref().clone())
    }

    #[inline]
    fn end(&self) -> Option<Self> {
        self.borrow().end.as_ref().map(|e| e.item_ref().clone())
    }
}

pub(crate) trait ItemIterator: Linked + StartEnd
where
    Self: Sized,
{
    #[inline]
    fn item_iter(&self) -> ItemIter<Self> {
        ItemIter { item: self.start() }
    }

    #[inline]
    fn visible_item_iter(&self) -> VisibleItemIter<Self> {
        VisibleItemIter { item: self.start() }
    }
}

impl<T: Linked + StartEnd> ItemIterator for T {}

pub(crate) struct ItemIter<T: Linked> {
    pub(crate) item: Option<T>,
}

pub(crate) struct VisibleItemIter<T: Linked> {
    pub(crate) item: Option<T>,
}

impl<T: Clone + StartEnd + Linked + WithId> Iterator for ItemIter<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.item.clone();
        self.item = item.as_ref().and_then(|i| i.right());

        item
    }
}

impl<T: Clone + StartEnd + Linked> Iterator for VisibleItemIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut item = self.item.clone();
        // move to right and return the first visible item
        while let Some(i) = item {
            // skip invisible items while moving to the right
            if i.is_visible() {
                self.item = i.right();
                return Some(i);
            }

            item = i.right();
        }

        None
    }
}

impl Deref for ItemRef {
    type Target = ItemRefInner;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl Serialize for ItemRef {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Type::from(self.clone()).serialize(serializer)
    }
}

impl Encode for ItemRef {
    #[inline]
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        self.borrow().data.encode(e, ctx);
    }
}

impl Decode for ItemRef {
    #[inline]
    fn decode<D: Decoder>(d: &mut D, _ctx: &DecodeContext) -> Result<Self, String> {
        Err("ItemRef::decode not implemented".to_string())
    }
}

impl WithId for ItemRef {
    #[inline]
    fn id(&self) -> Id {
        self.borrow().data.id
    }
}

impl WithTarget for ItemRef {
    #[inline]
    fn set_target(&self, target: Type) {
        self.borrow_mut().target = Some(target);
    }

    #[inline]
    fn get_target(&self) -> Option<Type> {
        self.borrow().target.clone()
    }
}

impl Eq for ItemRef {}

impl PartialEq<Self> for ItemRef {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.item
            .borrow()
            .id
            .compare_without_client(&other.item.borrow().id)
            == Ordering::Equal
    }
}

impl PartialOrd<Self> for ItemRef {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ItemRef {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.item.borrow().data.id.cmp(&other.item.borrow().data.id)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Item {
    pub(crate) flags: u8,
    pub(crate) data: ItemData,       // data for the item
    pub(crate) parent: Option<Type>, // parent link
    pub(crate) target: Option<Type>, // target link
    pub(crate) left: Option<Type>,   // left link
    pub(crate) right: Option<Type>,  // right link
    pub(crate) start: Option<Type>,  // linked children start
    pub(crate) end: Option<Type>,    // linked children end
    // pub(crate) marks: Option<Type>,   // linked marks
    // TODO: move the index to list to avoid per item allocation
    pub(crate) index: FractionalIndex, // runtime index for quick index lookup in a large list,
}

impl PartialEq<Content> for &Content {
    #[inline]
    fn eq(&self, other: &Content) -> bool {
        self.to_json().as_str() == other.to_json().as_str()
    }
}

impl Item {
    #[inline]
    pub(crate) fn new(data: ItemData) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }

    #[inline]
    pub(crate) fn is_moved(&self) -> bool {
        self.flags & 0x02 == 0x02
    }

    #[inline]
    pub(crate) fn is_deleted(&self) -> bool {
        if self.flags & 0x01 == 0x01 {
            return true;
        }

        // proxy and move items are considered deleted if the target is deleted
        match self.kind {
            ItemKind::Proxy | ItemKind::Move => {
                if let Some(ref target) = self.target {
                    target.is_deleted()
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    #[inline]
    pub(crate) fn is_inactive(&self) -> bool {
        self.flags & 0x04 == 0x04
    }

    pub(crate) fn field(&self, store: &WeakStoreRef) -> Option<String> {
        let store = store.upgrade().unwrap();
        let store = store.borrow();
        let field = store.get_field(&self.data.field.unwrap());
        field.map(|s| s.to_string())
    }

    #[inline]
    pub(crate) fn size(&self) -> u32 {
        match &self.data.content {
            Content::String(s) => s.len() as u32,
            Content::Mark(m) => m.size(),
            _ => 1,
        }
    }

    // #[inline]
    // pub(crate) fn container_id(&self) -> Option<Id> {
    //     self.container
    // }

    #[inline]
    pub(crate) fn parent(&self, store: &WeakStoreRef) -> Option<Type> {
        self.data
            .parent_id
            .and_then(|id| store.upgrade()?.borrow().find(&id))
    }

    #[inline]
    pub(crate) fn left_origin(&self, store: WeakStoreRef) -> Option<Type> {
        self.data
            .left_id
            .and_then(|id| store.upgrade()?.borrow().find(&id))
    }

    #[inline]
    pub(crate) fn right_origin(&self, store: WeakStoreRef) -> Option<Type> {
        self.data
            .right_id
            .and_then(|id| store.upgrade()?.borrow().find(&id))
    }

    #[inline]
    pub(crate) fn make_deleted(&mut self) {
        self.flags |= 0x01;
    }

    pub(crate) fn unmark_deleted(&mut self) {
        self.flags &= !0x01;
    }

    #[inline]
    pub(crate) fn mark_moved(&mut self) {
        self.flags |= 0x02;
    }

    #[inline]
    pub(crate) fn unmark_moved(&mut self) {
        self.flags &= !0x02;
    }

    #[inline]
    pub(crate) fn mark_inactive(&mut self) {
        self.flags |= 0x04;
    }

    #[inline]
    pub(crate) fn mark_active(&mut self) {
        self.flags &= !0x04;
    }

    pub(crate) fn set(&mut self, _key: &ItemKey, _ref: ItemRef) {}

    /// add mark to the item, see PeriText paper for details
    // pub(crate) fn add_mark(&mut self, mark: impl Into<Type>) {
    //     let mark = mark.into();
    //     if let Some(ref marks) = self.marks {
    //         let mut end = marks.clone();
    //         while end.right().is_some() {
    //             end = end.right().unwrap();
    //         }
    //
    //         end.set_right(mark)
    //     } else {
    //         self.marks = Some(mark);
    //     }
    // }

    #[inline]
    pub(crate) fn content(&self) -> Content {
        self.data.content.clone()
    }

    #[inline]
    pub(crate) fn content_mut(&mut self) -> &mut Content {
        &mut self.data.content
    }

    pub(crate) fn as_map(&self, store: &WeakStoreRef) -> HashMap<String, Type> {
        // TODO: use visible iterator for performance
        let items = self.items();
        let mut map = HashMap::new();

        for item in items.clone() {
            if let Some(field) = item.item_ref().borrow().field(store) {
                // println!("field: {}, id: {:?}", field, item.id());
                map.insert(field, item.clone());
            }
        }

        // remove items that are moved or deleted
        for item in items.iter() {
            if !item.is_visible() {
                map.remove(&item.item_ref().borrow().field(store).unwrap());
            }
        }

        map
    }

    // pub(crate) fn get_marks(&self) -> Vec<Type> {
    //     let mark_list = self.get_all_marks();
    //     let mut marks = HashMap::new();
    //
    //     for mark in mark_list {
    //         if let Content::Mark(mark_type) = mark.content() {
    //             marks.insert(mark_type.get_key(), mark);
    //         }
    //     }
    //
    //     for (field, mark) in marks.clone().iter() {
    //         if !mark.is_visible() {
    //             marks.remove(field);
    //         }
    //     }
    //
    //     marks.values().cloned().collect()
    // }

    // all marks need to match for adjacent string items to be merged into a single string
    // pub(crate) fn get_all_marks(&self) -> Vec<Type> {
    //     let mut mark_list: Vec<Type> = vec![];
    //     let mut mark = self.marks.clone();
    //
    //     while mark.is_some() {
    //         mark_list.push(mark.clone().unwrap());
    //         mark = mark.and_then(|m| m.right().clone());
    //     }
    //
    //     mark_list
    // }

    pub(crate) fn as_list(&self) -> Vec<Type> {
        let items = self.items();
        let mut list = vec![];

        for item in items.clone() {
            list.push(item.clone());
        }

        // remove items that are moved or deleted
        list.into_iter().filter(|item| item.is_visible()).collect()
    }

    pub(crate) fn items(&self) -> Vec<Type> {
        self.all_items()
            .into_iter()
            .filter(|item| item.is_visible())
            .collect()
    }

    pub(crate) fn all_items(&self) -> Vec<Type> {
        let mut items: Vec<Type> = vec![];
        let mut item = self.start.clone();
        while item.is_some() {
            items.push(item.clone().unwrap());
            item = item.and_then(|i| i.right().clone());
        }

        items
    }

    pub(crate) fn to_json(&self) -> IndexMap<String, Value> {
        let mut map = IndexMap::new();

        map.insert("id".to_string(), self.data.id.to_string().into());
        map.insert("kind".to_string(), self.data.kind.to_string().into());

        if let Some(parent) = &self.parent {
            map.insert("parent_id".to_string(), parent.id().to_string().into());
        }

        if let Some(left) = &self.left_id {
            map.insert("left_id".to_string(), left.id().to_string().into());
        }

        if let Some(right) = &self.right_id {
            map.insert("right_id".to_string(), right.id().to_string().into());
        }

        // map.insert("content".to_string(), self.data.content.to_json());

        map
    }

    pub(crate) fn serialize_with<S>(&self, s: &mut S) -> Result<(), S::Error>
    where
        S: serde::ser::SerializeStruct,
    {
        self.data.serialize_with(s)?;

        // let marks_map = self.get_marks();
        // let mut map = serde_json::Map::new();
        // for mark in marks_map.iter() {
        //     if let Content::Mark(mark) = mark.content() {
        //         let (k, v) = mark.get_key_value();
        //         map.insert(k, v);
        //     }
        // }
        // if !map.is_empty() {
        //     let marks = serde_json::to_value(map).unwrap_or_default();
        //     s.serialize_field("marks", &marks)?;
        // }

        Ok(())
    }

    pub(crate) fn serialize_size(&self) -> usize {
        let mut size = self.data.serialize_size();
        // let marks = self.get_marks();
        // if !marks.is_empty() {
        //     size += 1;
        // }

        size
    }
}

impl WithIdRange for ItemData {
    fn range(&self) -> IdRange {
        let id = self.id.clone();
        match self.kind {
            ItemKind::String => IdRange::new(id.client, id.clock, id.clock + self.ticks()),
            ItemKind::Mark => IdRange::new(id.client, id.clock, id.clock + self.ticks()),
            _ => IdRange::new(id.client, id.clock, id.clock),
        }
    }
}

impl Serialize for ItemData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("ItemData", self.serialize_size())?;
        self.serialize_with(&mut s)?;
        s.end()
    }
}

// pub(crate) trait OriginIds {
//     fn left_id(&self) -> Option<Id>;
//     fn right_id(&self) -> Option<Id>;
// }

impl Deref for Item {
    type Target = ItemData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Item {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum ItemSide {
    #[default]
    None,
    Left,
    Right,
}

bitflags! {
    pub(crate) struct ItemSideFlags: u8 {
        const NONE = 0x00;
        const LEFT = 0x01;
        const RIGHT = 0x02;
    }
}

impl ItemSide {
    pub(crate) fn is_left(&self) -> bool {
        matches!(self, ItemSide::Left)
    }

    pub(crate) fn is_right(&self) -> bool {
        matches!(self, ItemSide::Right)
    }

    pub(crate) fn is_none(&self) -> bool {
        matches!(self, ItemSide::None)
    }
}

impl From<ItemSide> for ItemSideFlags {
    fn from(kind: ItemSide) -> Self {
        match kind {
            ItemSide::None => ItemSideFlags::NONE,
            ItemSide::Left => ItemSideFlags::LEFT,
            ItemSide::Right => ItemSideFlags::RIGHT,
        }
    }
}

impl From<&ItemSide> for ItemSideFlags {
    fn from(kind: &ItemSide) -> Self {
        match kind {
            ItemSide::None => ItemSideFlags::NONE,
            ItemSide::Left => ItemSideFlags::LEFT,
            ItemSide::Right => ItemSideFlags::RIGHT,
        }
    }
}

impl From<ItemSideFlags> for ItemSide {
    fn from(flags: ItemSideFlags) -> Self {
        let flag = flags.bits();
        match flag {
            0 => ItemSide::None,
            1 => ItemSide::Left,
            2 => ItemSide::Right,
            _ => panic!("Invalid item side flag: {}", flag),
        }
    }
}

// item data is encoded and saved into persistent storage
#[derive(Debug, Clone, Default, Eq)]
pub struct ItemData {
    pub(crate) kind: ItemKind,
    pub(crate) id: Id,
    pub(crate) parent_id: Option<Id>,
    pub(crate) left_id: Option<Id>,
    pub(crate) right_id: Option<Id>,
    pub(crate) side: ItemSide,
    pub(crate) field: Option<FieldId>,
    pub(crate) content: Content,
}

impl ItemData {
    pub(crate) fn new(kind: ItemKind, id: Id) -> Self {
        Self {
            kind,
            id,
            parent_id: None,
            left_id: None,
            right_id: None,
            side: ItemSide::None,
            field: None,
            content: Content::Null,
        }
    }

    pub(crate) fn adjust(
        &self,
        before_clients: &ClientMap,
        before_fields: &FieldMap,
        after_clients: &ClientMap,
        after_fields: &FieldMap,
    ) -> ItemData {
        let mut data = self.clone();
        data.id = self.id.adjust(before_clients, after_clients);
        data.parent_id = data
            .parent_id
            .map(|id| id.adjust(before_clients, after_clients));
        data.left_id = data
            .left_id
            .map(|id| id.adjust(before_clients, after_clients));
        data.right_id = data
            .right_id
            .map(|id| id.adjust(before_clients, after_clients));

        let field = data.field.and_then(|field_id| {
            let field = before_fields.get_field(&field_id);
            field.and_then(|field| after_fields.get_field_id(field))
        });

        if let Some(field) = field {
            data.field = Some(*field);
        }

        data
    }

    #[inline]
    pub(crate) fn deps(&self) -> Vec<Id> {
        let mut deps = vec![];
        if let Some(parent_id) = &self.parent_id {
            deps.push(parent_id.clone().into());
        }

        if let Some(left_id) = &self.left_id {
            deps.push(left_id.clone().into());
        }

        if let Some(right_id) = &self.right_id {
            deps.push(right_id.clone().into());
        }

        deps
    }

    pub(crate) fn ticks(&self) -> u32 {
        match &self.content {
            Content::String(s) => s.len() as u32,
            Content::Mark(m) => m.size(),
            _ => 1,
        }
    }

    pub(crate) fn is_root(&self) -> bool {
        matches!(&self.content, Content::Doc(_))
    }

    pub(crate) fn serialize_with<S>(&self, s: &mut S) -> Result<(), S::Error>
    where
        S: serde::ser::SerializeStruct,
    {
        s.serialize_field("id", &self.id.to_string())?;
        s.serialize_field("kind", &self.kind.to_string())?;

        if let Some(parent) = &self.parent_id {
            s.serialize_field("parent_id", &parent.id().to_string())?;
        }

        if let Some(left) = &self.left_id {
            s.serialize_field("left_id", &left.id().to_string())?;
        }

        if let Some(right) = &self.right_id {
            s.serialize_field("right_id", &right.id().to_string())?;
        }

        if let Some(field) = &self.field {
            s.serialize_field("field", &field.to_string())?;
        }

        s.serialize_field("content", &self.content)?;

        Ok(())
    }

    pub(crate) fn serialize_size(&self) -> usize {
        let mut size = 2_usize;

        if self.parent_id.is_some() {
            size += 1;
        }

        if self.left_id.is_some() {
            size += 1;
        }

        if self.right_id.is_some() {
            size += 1;
        }

        if self.field.is_some() {
            size += 1;
        }

        size
    }
}

impl Optimize for ItemData {
    fn optimize(&mut self) {
        if self.left_id.is_some() && self.parent_id.is_some() {
            self.parent_id = None;
        }
    }
}

pub(crate) trait Optimize {
    fn optimize(&mut self);
}

impl Split for ItemData {
    type Target = ItemData;
    fn split(&self, offset: u32) -> Result<(Self, Self), String> {
        let mut left = self.clone();
        let mut right = self.clone();

        match self.kind {
            ItemKind::String | ItemKind::Mark => {
                // do nothing
            }
            _ => return Err(stringify!("Cannot split {} item", self.kind).to_string()),
        }

        let size = match &self.content {
            Content::String(s) => s.len() as u32,
            Content::Mark(m) => m.size(),
            _ => return Err("Cannot split non-string item".to_string()),
        };

        // split id
        let (left_range, right_range) = self.id.range(size).split(offset)?;
        left.id = left_range.start_id();
        right.id = right_range.start_id();

        left.right_id = Some(right_range.start_id());
        right.left_id = Some(left_range.end_id());

        match &self.content {
            Content::String(s) => {
                let (l, r) = s.split_at(offset as usize);
                left.content = Content::String(l.to_string());
                right.content = Content::String(r.to_string());
            }
            Content::Mark(m) => {
                let (l, r) = m.split(offset);
                left.content = Content::Mark(l);
                right.content = Content::Mark(r);
            }
            _ => return Err("Cannot split non-string item".to_string()),
        }

        Ok((left, right))
    }
}

impl PartialEq for ItemData {
    fn eq(&self, other: &Self) -> bool {
        if self.kind != other.kind {
            return false;
        }

        if self.id != other.id {
            return false;
        }

        // item with left_id might not store parent_id, so we need to compare left_id
        if self.parent_id != other.parent_id && self.left_id != other.left_id {
            return false;
        }

        if self.right_id != other.right_id {
            return false;
        }

        if self.field != other.field {
            return false;
        }

        if self.content != other.content {
            return false;
        }

        true
    }
}

impl From<ItemData> for Item {
    fn from(data: ItemData) -> Self {
        Self::new(data)
    }
}

impl From<ItemData> for ItemRefInner {
    fn from(data: ItemData) -> Self {
        Rc::new(RefCell::new(Item::new(data)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Map,
    List,
    Text,
    PlainText,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub(crate) enum ItemKind {
    Map,
    List,
    Text,
    String,
    Atom,
    Proxy,
    Move,
    Mark,
    PlaintText,
}

impl ItemKind {
    #[inline]
    pub(crate) fn is_list(&self) -> bool {
        self == &Self::List
    }

    pub(crate) fn is_map(&self) -> bool {
        self == &Self::Map
    }

    pub(crate) fn is_text(&self) -> bool {
        self == &Self::Text
    }

    pub(crate) fn is_string(&self) -> bool {
        self == &Self::String
    }

    pub(crate) fn is_atom(&self) -> bool {
        self == &Self::Atom
    }

    pub(crate) fn is_proxy(&self) -> bool {
        self == &Self::Proxy
    }

    #[inline]
    pub(crate) fn is_move(&self) -> bool {
        self == &Self::Move
    }

    pub(crate) fn is_mark(&self) -> bool {
        self == &Self::Mark
    }

    pub(crate) fn is_plaintext(&self) -> bool {
        self == &Self::PlaintText
    }
}

bitflags! {
    pub(crate) struct ItemKindFlags: u8 {
        const MAP = 0x0;
        const LIST = 0x1;
        const TEXT = 0x2;
        const STRING = 0x3;
        const ATOM = 0x4;
        const PROXY = 0x5;
        const MOVE = 0x6;
        const MARK = 0x7;
        const PLAINTEXT = 0x8;
    }
}

impl From<ItemKind> for ItemKindFlags {
    fn from(kind: ItemKind) -> Self {
        match kind {
            ItemKind::Map => Self::MAP,
            ItemKind::List => Self::LIST,
            ItemKind::Text => Self::TEXT,
            ItemKind::String => Self::STRING,
            ItemKind::Atom => Self::ATOM,
            ItemKind::Proxy => Self::PROXY,
            ItemKind::Move => Self::MOVE,
            ItemKind::Mark => Self::MARK,
            ItemKind::PlaintText => Self::PLAINTEXT,
        }
    }
}

impl From<&ItemKind> for ItemKindFlags {
    fn from(kind: &ItemKind) -> Self {
        match kind {
            ItemKind::Atom => Self::ATOM,
            ItemKind::Map => Self::MAP,
            ItemKind::List => Self::LIST,
            ItemKind::Text => Self::TEXT,
            ItemKind::String => Self::STRING,
            ItemKind::Proxy => Self::PROXY,
            ItemKind::Move => Self::MOVE,
            ItemKind::Mark => Self::MARK,
            ItemKind::PlaintText => Self::PLAINTEXT,
        }
    }
}

impl From<ItemKindFlags> for ItemKind {
    fn from(flags: ItemKindFlags) -> Self {
        let flag = flags.bits();
        match flag {
            0x00 => ItemKind::Map,
            0x01 => ItemKind::List,
            0x02 => ItemKind::Text,
            0x03 => ItemKind::String,
            0x04 => ItemKind::Atom,
            0x05 => ItemKind::Proxy,
            0x06 => ItemKind::Move,
            0x07 => ItemKind::Mark,
            0x08 => ItemKind::PlaintText,
            _ => ItemKind::Atom,
        }
    }
}

impl Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Map => write!(f, "map"),
            Self::List => write!(f, "list"),
            Self::Text => write!(f, "text"),
            Self::String => write!(f, "string"),
            Self::Atom => write!(f, "atom"),
            Self::Proxy => write!(f, "proxy"),
            Self::Move => write!(f, "move"),
            Self::Mark => write!(f, "mark"),
            Self::PlaintText => write!(f, "plaintext"),
        }
    }
}

impl Default for ItemKind {
    fn default() -> Self {
        Self::Atom
    }
}

impl WithId for ItemData {
    #[inline]
    fn id(&self) -> Id {
        self.id
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ItemKey {
    Number(u32),
    String(String),
}

impl ItemKey {
    pub(crate) fn as_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Number(n) => n.to_string(),
        }
    }
}

impl From<&str> for ItemKey {
    fn from(s: &str) -> Self {
        Self::String(String::from(s))
    }
}

impl From<String> for ItemKey {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<usize> for ItemKey {
    fn from(n: usize) -> Self {
        Self::Number(n as u32)
    }
}

impl From<u32> for ItemKey {
    fn from(n: u32) -> Self {
        Self::Number(n as u32)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Content {
    Doc(DocProps),
    Id(Id),           // id of the item
    Types(Vec<Type>), // list of types, backbone for crdt types
    Mark(MarkContent),
    Binary(Vec<u8>),
    String(String),
    Embed(Any),
    Null,
}

bitflags! {
    pub(crate) struct ContentFlags: u8 {
        const MARK = 0x00;
        const BINARY = 0x01;
        const STRING = 0x02;
        const TYPES = 0x03;
        const EMBED = 0x10;
        const DOC = 0x11;
        const NULL = 0x12;
        const ID = 0x13;
    }
}

impl Content {
    pub(crate) fn to_json(&self) -> Value {
        match self {
            Self::Mark(m) => Value::String(serde_json::to_string(m).unwrap()),
            Self::Binary(b) => Value::String(serde_json::to_string(b).unwrap()),
            Self::String(s) => Value::String(s.clone()),
            Self::Types(t) => Value::Array(t.iter().map(|t| t.to_json()).collect()),
            Self::Embed(a) => a.to_json(),
            Self::Doc(d) => Value::String(serde_json::to_string(&d.id).unwrap()),
            Self::Id(id) => Value::String(id.to_string()),
            Self::Null => Value::Null,
        }
    }
}

impl Serialize for Content {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            Self::Binary(b) => serializer.serialize_str(&serde_json::to_string(b).unwrap()),
            Self::String(s) => serializer.serialize_str(s),
            // Self::Embed(a) => a.serialize(serializer),
            Self::Doc(d) => serializer.serialize_str(&serde_json::to_string(&d.id).unwrap()),
            Self::Null => serializer.serialize_none(),
            Self::Types(list) => {
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for item in list {
                    seq.serialize_element(&item.content())?;
                }
                seq.end()
            }
            _ => serializer.serialize_none(),
        }
    }
}

impl Encode for Content {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        match self {
            Self::Mark(m) => {
                e.u8(ContentFlags::MARK.bits());
                m.encode(e, ctx)
            }
            Self::Binary(b) => {
                e.u8(ContentFlags::BINARY.bits());
                e.bytes(b)
            }
            Self::String(s) => {
                e.u8(ContentFlags::STRING.bits());
                e.string(s)
            }
            Self::Types(_) => {
                // e.array(t)
            }
            Self::Embed(_) => {
                // a.encode(e)
            }
            Self::Doc(d) => {
                e.u8(ContentFlags::DOC.bits());
                d.encode(e, ctx)
            }
            Self::Id(id) => {
                e.u8(ContentFlags::ID.bits());
                id.encode(e, ctx)
            }
            Self::Null => {}
        }
    }
}

impl Decode for Content {
    fn decode<T: Decoder>(d: &mut T, ctx: &DecodeContext) -> Result<Self, String>
    where
        Self: Sized,
    {
        let flags = d.u8()?;

        match flags {
            0x00 => Ok(Self::Mark(MarkContent::decode(d, ctx)?)),
            0x01 => Ok(Self::Binary(d.bytes()?)),
            0x02 => Ok(Self::String(d.string()?)),
            0x03 => {
                let len = d.u32()? as usize;
                let mut types = vec![];
                for _ in 0..len {
                    types.push(Type::decode(d, ctx)?);
                }
                Ok(Self::Types(types))
            }
            0x10 => {
                // let any = Any::decode(d, ctx)?;
                // Ok(Self::Embed(any))
                Ok(Self::Null)
            }
            0x11 => {
                let doc = DocProps::decode(d, ctx)?;
                Ok(Self::Doc(doc))
            }
            0x12 => Ok(Self::Null),
            0x13 => Ok(Self::Id(Id::decode(d, ctx)?)),
            _ => Err(format!("Invalid content flags: {}", flags)),
        }
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::Null
    }
}

impl From<MarkContent> for Content {
    fn from(m: MarkContent) -> Self {
        Self::Mark(m)
    }
}

impl From<&String> for Content {
    fn from(s: &std::string::String) -> Self {
        Self::String(s.to_string())
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<Vec<u8>> for Content {
    fn from(b: Vec<u8>) -> Self {
        Self::Binary(b)
    }
}

impl From<u32> for Content {
    fn from(b: u32) -> Self {
        Self::Embed(U32(b))
    }
}

impl From<Vec<Type>> for Content {
    fn from(t: Vec<Type>) -> Self {
        Self::Types(t)
    }
}

impl From<DocProps> for Content {
    fn from(d: DocProps) -> Self {
        Self::Doc(d)
    }
}

impl From<Any> for Content {
    fn from(a: Any) -> Self {
        Self::Embed(a)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct DocProps {
    pub(crate) id: DocId,
    pub(crate) created_at: u64,
    // user id of the creator
    pub(crate) created_by: Client,
    // custom create time props fot the document
    pub(crate) props: Any,
}

impl DocProps {
    pub(crate) fn new(guid: DocId, created_by: Client) -> Self {
        Self {
            id: guid,
            created_by,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            props: Any::Null,
        }
    }
}

impl Encode for DocProps {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        self.id.encode(e, ctx);
        e.u64(self.created_at);
        self.created_by.encode(e, ctx);
        self.props.encode(e, ctx);
    }
}

impl Decode for DocProps {
    fn decode<T: Decoder>(d: &mut T, ctx: &DecodeContext) -> Result<Self, String>
    where
        Self: Sized,
    {
        let doc_id = DocId::decode(d, ctx)?;
        let created_at = d.u64()?;
        let created_by = Client::decode(d, ctx)?;
        let props = Any::decode(d, ctx)?;

        Ok(Self {
            id: doc_id,
            created_at: created_at.into(),
            created_by,
            props,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum Any {
    #[default]
    Null,
    True,
    False,
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    String(String),
    Binary(Vec<u8>),
    Array(Vec<Any>),
    Map(Vec<(String, Any)>),
    KV(Vec<(String, String)>),
}

impl Any {
    pub(crate) fn to_json(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::True => Value::Bool(true),
            Self::False => Value::Bool(false),
            Self::F32(f) => Value::Number(serde_json::Number::from_f64(*f as f64).unwrap()),
            Self::F64(f) => Value::Number(serde_json::Number::from_f64(*f).unwrap()),
            Self::I8(i) => Value::Number(serde_json::Number::from(*i)),
            Self::I16(i) => Value::Number(serde_json::Number::from(*i)),
            Self::I32(i) => Value::Number(serde_json::Number::from(*i)),
            Self::I64(i) => Value::Number(serde_json::Number::from(*i)),
            Self::U8(u) => Value::Number(serde_json::Number::from(*u)),
            Self::U16(u) => Value::Number(serde_json::Number::from(*u)),
            Self::U32(u) => Value::Number(serde_json::Number::from(*u)),
            Self::U64(u) => Value::Number(serde_json::Number::from(*u)),
            Self::String(s) => Value::String(s.clone()),
            Self::Binary(b) => Value::String(serde_json::to_string(b).unwrap()),
            Self::Array(a) => Value::Array(a.iter().map(|a| a.to_json()).collect()),
            Self::Map(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m.iter() {
                    map.insert(k.clone(), v.to_json());
                }
                Value::Object(map)
            }
            Self::KV(kv) => {
                let mut map = serde_json::Map::new();
                for (k, v) in kv.iter() {
                    map.insert(k.clone(), Value::String(v.clone()));
                }
                Value::Object(map)
            }
        }
    }

    pub(crate) fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub(crate) fn into_kv_map(self) -> HashMap<String, String> {
        if let Self::KV(kv) = self {
            kv.into_iter().collect()
        } else {
            HashMap::new()
        }
    }
}

bitflags! {
    pub(crate) struct AnyFlags: u8 {
        const NULL = 0x00;
        const TRUE = 0x01;
        const FALSE = 0x02;
        const FLOAT32 = 0x03;
        const FLOAT64 = 0x04;
        const INT8 = 0x05;
        const INT16 = 0x06;
        const INT32 = 0x07;
        const INT64 = 0x08;
        const UINT8 = 0x09;
        const UINT16 = 0x0A;
        const UINT32 = 0x0B;
        const UINT64 = 0x0C;
        const STRING = 0x0D;
        const BINARY = 0x0E;
        const ARRAY = 0x0F;
        const MAP = 0x10;
    }
}

impl Encode for Any {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        match self {
            Any::Null => {
                e.u8(AnyFlags::NULL.bits());
            }
            Any::True => {
                e.u8(AnyFlags::TRUE.bits());
            }
            Any::False => {
                e.u8(AnyFlags::FALSE.bits());
            }
            Any::F32(_) => {
                e.u8(AnyFlags::FLOAT32.bits());
                // e.f32(*d);
            }
            Any::F64(d_) => {
                e.u8(AnyFlags::FLOAT64.bits());
                // e.f64(*d);
            }
            Any::I8(_) => {}
            Any::I16(_) => {}
            Any::I32(_) => {}
            Any::I64(_) => {}
            Any::U8(_) => {}
            Any::U16(_) => {}
            Any::U32(_) => {}
            Any::U64(_) => {}
            Any::String(_) => {}
            Any::Binary(_) => {}
            Any::Array(_) => {}
            Any::Map(_) => {}
            Any::KV(_) => {}
        }
    }
}

impl Decode for Any {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<Self, String>
    where
        Self: Sized,
    {
        let flags = d.u8()?;
        match flags {
            0x00 => Ok(Self::Null),
            0x01 => Ok(Self::True),
            0x02 => Ok(Self::False),
            0x03 => {
                // let f = d.f32()?;
                // Ok(Self::Float32(f))
                Ok(Self::Null)
            }
            0x04 => {
                // let f = d.f64()?;
                // Ok(Self::Float64(f))
                Ok(Self::Null)
            }
            0x05 => {
                // let i = d.i8()?;
                // Ok(Self::Int8(i))
                Ok(Self::Null)
            }
            0x06 => {
                // let i = d.i16()?;
                // Ok(Self::Int16(i))
                Ok(Self::Null)
            }
            0x07 => {
                // let i = d.i32()?;
                // Ok(Self::Int32(i))
                Ok(Self::Null)
            }
            0x08 => {
                // let i = d.i64()?;
                // Ok(Self::Int64(i))
                Ok(Self::Null)
            }
            0x09 => {
                // let u = d.u8()?;
                // Ok(Self::Uint8(u))
                Ok(Self::Null)
            }
            0x0A => {
                // let u = d.u16()?;
                // Ok(Self::Uint16(u))
                Ok(Self::Null)
            }
            0x0B => {
                // let u = d.u32()?;
                // Ok(Self::Uint32(u))
                Ok(Self::Null)
            }
            0x0C => {
                // let u = d.u64()?;
                // Ok(Self::Uint64(u))
                Ok(Self::Null)
            }
            0x0D => {
                // let s = d.string()?;
                // Ok(Self::String(s))
                Ok(Self::Null)
            }
            0x0E => {
                // let b = d.bytes()?;
                // Ok(Self::Binary(b))
                Ok(Self::Null)
            }
            0x0F => {
                panic!("Array not implemented");
            }
            _ => {
                panic!("Map not implemented");
            }
        }
    }
}

impl Eq for Any {}

#[cfg(test)]
mod tests {
    use crate::natom::NAtom;
    use crate::nlist::NList;
    use crate::nmap::NMap;
    use crate::nmark::NMark;
    use crate::nmove::NMove;
    use crate::*;
    use fractional_index::FractionalIndex;
    use std::rc::Rc;

    #[test]
    fn test_option_size() {
        let item: Option<Type> = None;
        println!("{:?}", size_of::<Item>());
        println!("{:?}", size_of::<ItemData>());
        println!("{:?}", size_of::<Type>());
        println!("{:?}", size_of::<[Type; 6]>());
        println!("{:?}", size_of::<Content>());
        println!("{:?}", size_of::<FractionalIndex>());
        println!("{:?}", size_of::<ItemRef>());
        println!("{:?}", size_of::<ItemSide>());
        println!("{:?}", size_of::<Id>());
        println!("{:?}", size_of::<ItemKind>());

        println!("{:?}", size_of::<NList>());
        println!("{:?}", size_of::<NMap>());
        println!("{:?}", size_of::<NString>());
        println!("{:?}", size_of::<NMark>());
        println!("{:?}", size_of::<NMove>());
        println!("{:?}", size_of::<NAtom>());
        println!("{:?}", size_of::<NText>());

        enum Num {
            U32(u32),
            U64(u64),
        }
        println!("{:?}", size_of::<Num>());
        println!("{:?}", size_of::<Vec<u32>>());
    }
}
