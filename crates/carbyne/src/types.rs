use fake::Opt;
use fractional_index::FractionalIndex;
use serde::Serialize;
use serde_json::Value;
use std::cmp::Ordering;

use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::delete::DeleteItem;
use crate::doc::{Doc, DocMeta};
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKey, ItemKind, ItemRef, Linked, StartEnd, WithIndex};
use crate::mark::Mark;
use crate::natom::NAtom;
use crate::nlist::NList;
use crate::nmap::NMap;
use crate::nmark::NMark;
use crate::nmove::NMove;
use crate::nstring::NString;
use crate::ntext::NText;
use crate::store::{StoreRef, WeakStoreRef};
use crate::{print_yaml, Client};

/// Type is a wrapper around the different item types in the store.
#[derive(Debug, Clone, Default)]
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
    pub(crate) fn data(&self) -> ItemData {
        self.item_ref().borrow().data.clone()
    }
}

impl Type {
    pub(crate) fn replace(&self, items: (Type, Type)) {
        let store = self.store().upgrade().unwrap();
        let mut store = store.borrow_mut();

        if let Some(left) = self.left() {
            left.set_right(Some(items.0.clone()));
        }

        if let Some(right) = self.right() {
            right.set_left(Some(items.1.clone()));
        }

        if let Some(parent) = self.parent() {
            items.0.set_parent(Some(parent.clone()));
            items.1.set_parent(Some(parent.clone()));
        }

        store.items.replace(&self.clone(), items);
    }
}

impl Type {
    pub(crate) fn slice(&self, start: u32, end: u32) -> Vec<Type> {
        match self {
            Type::Text(n) => n.slice(start, end),
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

    #[inline]
    pub(crate) fn store(&self) -> WeakStoreRef {
        self.item_ref().store.clone()
    }

    #[inline]
    pub(crate) fn depth(&self) -> u32 {
        self.item_ref().depth()
    }

    #[inline]
    pub(crate) fn right_origin(&self) -> Option<Type> {
        self.item_ref().borrow().right_origin(self.store())
    }

    #[inline]
    pub(crate) fn left_origin(&self) -> Option<Type> {
        self.item_ref().borrow().left_origin(self.store())
    }

    #[inline]
    pub(crate) fn parent(&self) -> Option<Type> {
        self.item_ref().borrow().parent.clone()
    }

    #[inline]
    pub(crate) fn parent_id(&self) -> Option<Id> {
        self.item_ref().borrow().data.parent_id
    }

    #[inline]
    pub(crate) fn left_id(&self) -> Option<Id> {
        self.item_ref().borrow().data.left_id
    }

    #[inline]
    pub(crate) fn right_id(&self) -> Option<Id> {
        self.item_ref().borrow().data.right_id
    }

    #[inline]
    pub(crate) fn start_id(&self) -> Id {
        self.item_ref().id().range(1).start_id()
    }

    #[inline]
    pub(crate) fn end_id(&self) -> Id {
        if let ItemKind::String = self.kind() {
            self.item_ref().id().range(self.size()).end_id()
        } else {
            self.item_ref().id().range(1).end_id()
        }
    }

    #[inline]
    pub(crate) fn set_parent(&self, parent: impl Into<Option<Type>>) {
        self.item_ref()
            .borrow_mut()
            .parent
            .clone_from(&parent.into());
    }

    #[inline]
    pub(crate) fn set_parent_id(&self, parent_id: impl Into<Option<Id>>) {
        self.item_ref()
            .borrow_mut()
            .parent_id
            .clone_from(&parent_id.into());
    }

    #[inline]
    pub(crate) fn set_left(&self, left: impl Into<Option<Type>>) {
        self.item_ref().borrow_mut().left.clone_from(&left.into());
    }

    #[inline]
    pub(crate) fn set_left_id(&self, left_id: impl Into<Option<Id>>) {
        self.item_ref()
            .borrow_mut()
            .left_id
            .clone_from(&left_id.into());
    }

    #[inline]
    pub(crate) fn set_right(&self, right: impl Into<Option<Type>>) {
        self.item_ref().borrow_mut().right.clone_from(&right.into());
    }

    #[inline]
    pub(crate) fn set_right_id(&self, right_id: impl Into<Option<Id>>) {
        self.item_ref()
            .borrow_mut()
            .right_id
            .clone_from(&right_id.into());
    }

    #[inline]
    pub(crate) fn set_start(&self, start: impl Into<Option<Type>>) {
        self.item_ref().borrow_mut().start.clone_from(&start.into());
    }

    #[inline]
    pub(crate) fn set_end(&self, end: impl Into<Option<Type>>) {
        self.item_ref().borrow_mut().end.clone_from(&end.into());
    }

    // insert after skips the list index lookup and directly inserts the item after the current item
    pub fn insert_after(&self, item: impl Into<Type>) {
        let item = item.into();

        let parent = self.parent();
        let next = self.right();

        item.set_parent_id(parent.as_ref().map(|p| p.id()));
        item.set_left_id(Some(self.id()));
        item.set_right_id(next.as_ref().map(|n| n.id()));

        // item.set_parent(parent.clone());
        item.set_left(self.clone());
        item.set_right(next.clone());

        self.set_right(item.clone());

        if let Some(next) = next {
            next.set_left(item.clone());
        } else if let Some(ref parent) = parent {
            parent.set_end(item.clone());
        }
    }

    // insert before skips the list index lookup and directly inserts the item before the current item
    pub fn insert_before(&self, item: Type) {
        let parent = self.parent();
        let prev = self.left();

        item.set_parent_id(parent.as_ref().map(|p| p.id()));
        item.set_left_id(prev.as_ref().map(|p| p.id()));
        item.set_right_id(Some(self.id()));

        item.set_parent(parent.clone());
        item.set_left(prev.clone());
        item.set_right(self.clone());

        self.set_left(item.clone());

        if let Some(prev) = prev {
            prev.set_right(item.clone());
        } else if let Some(ref parent) = parent {
            parent.set_start(item.clone());
        }

        parent.unwrap().on_insert(&item);
    }

    pub(crate) fn item_ref(&self) -> ItemRef {
        match self {
            Type::List(n) => n.item_ref(),
            Type::Map(n) => n.item_ref(),
            Type::Text(n) => n.item_ref(),
            Type::String(n) => n.item_ref(),
            Type::Atom(n) => n.item_ref(),
            Type::Move(n) => n.item_ref(),
            Type::Mark(n) => n.item_ref(),
            // Type::Doc(n) => n.root.item_ref(),
            Type::Identity => panic!("item_ref: not implemented"),
        }
    }

    // get the container of the item
    // the container contains the item
    // pub(crate) fn container(&self) -> Option<Type> {
    //     if let Some(container_id) = self.container_id() {
    //         let typ = self.store().upgrade().and_then(|store| {
    //             let container = store.borrow().items.get(&container_id);
    //             return container.map_or(None, |container| Some(container.clone()));
    //         });
    //
    //         return typ;
    //     }
    //
    //     None
    // }

    // #[inline]
    // pub(crate) fn container_id(&self) -> Option<Id> {
    //     self.item_ref().borrow().container.clone()
    // }
    //
    // #[inline]
    // pub(crate) fn set_container_id(&self, container_id: impl Into<Option<Id>>) {
    //     self.item_ref().borrow_mut().container = container_id.into();
    // }

    #[inline]
    pub(crate) fn is_moved(&self) -> bool {
        self.item_ref().borrow().is_moved()
    }

    #[inline]
    pub(crate) fn is_deleted(&self) -> bool {
        self.item_ref().borrow().is_deleted()
    }

    #[inline]
    pub(crate) fn is_inactive(&self) -> bool {
        self.item_ref().borrow().is_inactive()
    }

    #[inline]
    pub(crate) fn is_visible(&self) -> bool {
        !self.is_moved() && !self.is_deleted() && !self.is_inactive()
    }

    #[inline]
    pub(crate) fn index_of(&self, target: &Type) -> i32 {
        match self {
            Type::List(list) => list.index_of(target),
            _ => -1,
        }
    }

    #[inline]
    pub fn kind(&self) -> ItemKind {
        self.item_ref().kind()
    }

    #[inline]
    pub fn field(&self) -> Option<String> {
        self.item_ref().field()
    }

    // disconnect from the document tree
    pub(crate) fn disconnect(&self) {
        let parent = self.parent();
        // print_yaml(&self);
        match self.parent() {
            Some(Type::List(n)) => n.remove_child(self),
            Some(Type::Map(n)) => n.remove_child(self),
            // Type::Text(n) => n.add_mark(mark),
            Some(parent) => panic!("disconnect is not implemented for {}", parent.kind()),
            _ => panic!("disconnect called for an orphan item"),
        }
    }

    pub(crate) fn add_mark(&self, mark: Mark) {
        match self {
            // Type::List(n) => n.add_mark(mark),
            Type::Map(n) => n.add_mark(mark),
            // Type::Text(n) => n.add_mark(mark),
            Type::String(n) => n.add_mark(mark),
            // Type::Atom(n) => n.add_mark(mark),
            // Type::Move(n) => n.add_mark(mark),
            Type::Identity => panic!("add_mark: not implemented for identity"),
            _ => panic!("add_mark: not implemented"),
        }
    }

    #[inline]
    pub(crate) fn remove_mark(&self, mark: Mark) {
        let id = self.store().upgrade().unwrap().borrow_mut().next_id();
        // let marks = self.item_ref().borrow().get_marks();
        let item = DeleteItem::new(id, self.range());
    }

    #[inline]
    pub fn size(&self) -> u32 {
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

    pub fn content(&self) -> Content {
        match self {
            Type::String(n) => n.content(),
            Type::Atom(n) => n.content(),
            Type::Text(n) => n.content(),
            Type::Move(n) => n.content(),
            Type::Mark(n) => n.content(),
            Type::List(n) => n.content(),
            Type::Map(n) => n.content(),
            _ => {
                panic!("content: not implemented for {:?}", self.kind())
            }
        }
    }

    /// move the item to the given parent at the given offset
    pub fn move_to(&self, parent: impl Into<Type>, offset: u32) {
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
        if let Some(parent) = before.parent() {
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
        if let Some(parent) = after.parent() {
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
    pub fn insert(&self, offset: u32, item: impl Into<Type>) {
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
        self.item_ref().delete(1);
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
            Type::Atom(n) => n.text_content(),
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
    pub(crate) fn split(&self, offset: u32) -> (Type, Type) {
        match self {
            Type::String(n) => n.split(offset).unwrap(),
            Type::Mark(n) => n.split(offset).unwrap(),
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
            Type::List(n) => {
                Self::add_frac_index(child);
                n.on_insert(child)
            }
            Type::Text(n) => {
                // Self::add_frac_index(child);
                n.on_insert(child)
            }
            Type::Map(n) => {}
            _ => panic!("on_insert: not implemented for {:?}", self.kind()),
        }
    }

    pub(crate) fn add_frac_index(&self) {
        let index = match (self.left(), self.right()) {
            (Some(left), Some(right)) => {
                FractionalIndex::new_between(&left.index(), &right.index()).unwrap()
            }
            (Some(left), None) => FractionalIndex::new_after(&left.index()),
            (None, Some(right)) => FractionalIndex::new_before(&right.index()),
            (None, None) => FractionalIndex::default(),
        };

        self.item_ref().borrow_mut().index = index;
    }

    //
    pub(crate) fn rollback(&self) {}

    pub(crate) fn on_delete(&self, child: &Type) {}

    pub(crate) fn on_undelete(&self, child: &Type) {}

    pub(crate) fn on_move(&self, child: &Type) {}

    pub(crate) fn on_unmove(&self, child: &Type) {}
}

impl WithIndex for Type {
    fn index(&self) -> FractionalIndex {
        self.item_ref().index()
    }
}

impl Linked for Type {
    #[inline]
    fn left(&self) -> Option<Type> {
        self.item_ref().borrow().left.clone()
    }

    #[inline]
    fn right(&self) -> Option<Type> {
        self.item_ref().borrow().right.clone()
    }

    #[inline]
    fn is_visible(&self) -> bool {
        !self.is_deleted() || !self.is_moved()
    }
}

impl StartEnd for Type {
    #[inline]
    fn start(&self) -> Option<Type> {
        self.item_ref().borrow().start.clone()
    }

    #[inline]
    fn end(&self) -> Option<Type> {
        self.item_ref().borrow().end.clone()
    }
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
        e.item(ctx, &self.item_ref().borrow().data.clone());
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
        self.item_ref().id()
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
        // if let Content::Doc(d) = n.content() {
        //     Type::Doc(Doc::new(DocMeta {
        //         id: d.id,
        //         created_at: d.created_at,
        //         crated_by: Client::from(d.created_by),
        //         props: d.props.into_kv_map(),
        //     }))
        // } else {
        Self::Map(n)
        // }
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

impl From<ItemRef> for Type {
    fn from(item: ItemRef) -> Self {
        let kind = item.borrow().kind.clone();
        match kind {
            ItemKind::List => Self::List(item.into()),
            ItemKind::Map => Self::Map(item.into()),
            ItemKind::Text => Self::Text(item.into()),
            ItemKind::String => Self::String(item.into()),
            ItemKind::Atom => Self::Atom(item.into()),
            ItemKind::Move => Self::Move(item.into()),
            ItemKind::Mark => Self::Mark(item.into()),
            _ => panic!("Type::from(ItemRef): not implemented"),
        }
    }
}

impl From<Type> for ItemRef {
    fn from(t: Type) -> Self {
        match t {
            Type::List(n) => n.item_ref(),
            Type::Map(n) => n.item_ref(),
            Type::Text(n) => n.item_ref(),
            Type::String(n) => n.item_ref(),
            Type::Atom(n) => n.item_ref(),
            Type::Move(n) => n.item_ref(),
            Type::Mark(n) => n.item_ref(),
            // Type::Doc(n) => n.root.item_ref(),
            Type::Identity => panic!("Type::into(ItemRef): not implemented"),
        }
    }
}

impl<T: Into<Type> + Clone> From<&T> for Type {
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        let store = self.store().upgrade().unwrap();
        let store = store.borrow();
        self.id().compare(&other.id(), &store.state.clients)
        // self.id().compare(&other.id(), &store.state.clients)
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
