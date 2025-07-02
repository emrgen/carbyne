use crate::bimapid::FieldId;
use crate::cycle::creates_cycle;
use crate::delete::DeleteItem;
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, WithId, WithIdRange, WithTarget};
use crate::index::{BTreeIndex, IBTree, ItemIndexMap};
use crate::index_tree;
use crate::index_tree::{IndexTree, ValueIter};
use crate::inner_type::InnerType;
use crate::item::{Content, ItemData, ItemKey, ItemKind};
use crate::natom::{AItem, InnerAtom, NAtom};
use crate::nmove::NMove;
use crate::store::{DocStore, WeakStoreRef};
use crate::types::Type;
use log::warn;
use serde::ser::{Serialize, SerializeStruct};
use std::cell::{RefCell, RefMut};
use std::collections::vec_deque::Iter;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, Index};
use std::rc::Rc;

// expose a vec like api for get, insert, remove, index_of
#[derive(Debug)]
pub(crate) enum ListContent<T: Debug + Clone + Eq + Hash> {
    Small(Vec<T>),
    Large(IndexTree<T>),
}

impl<T: Debug + Clone + Eq + Hash> Default for ListContent<T> {
    fn default() -> Self {
        ListContent::Small(Vec::new())
    }
}

impl<T: Debug + Clone + Eq + Hash> ListContent<T> {
    fn get(&self, index: usize) -> Option<&T> {
        match self {
            ListContent::Small(v) => v.get(index),
            ListContent::Large(v) => v.at_index(index),
        }
    }

    fn is_small(&self) -> bool {
        match self {
            ListContent::Small(_) => true,
            ListContent::Large(_) => false,
        }
    }

    // auto upgrade to index tree when item count reach above a threshold
    fn insert(&mut self, index: usize, item: T) {
        // the small container size limit is a runtime config
        if self.is_small() && self.size() == 16 {
            let mut tree = IndexTree::new();
            match self {
                ListContent::Small(v) => v.drain(0..).enumerate().for_each(|(i, t)| {
                    tree.insert(i, t);
                }),
                _ => {}
            };

            *self = ListContent::Large(tree)
        }

        match self {
            ListContent::Small(v) => {
                v.insert(index, item);
            }
            ListContent::Large(v) => {
                v.insert(index, item);
            }
        }
    }

    // auto downgrade to vec when item count reach below a threshold
    pub(crate) fn remove(&self, index: usize) -> Option<&T> {
        None
    }

    pub(crate) fn first(&self) -> Option<&T> {
        self.get(0)
    }

    pub(crate) fn last(&self) -> Option<&T> {
        self.get(self.size() - 1)
    }

    pub(crate) fn append(&mut self, item: T) {
        self.insert(self.size(), item);
    }

    pub(crate) fn prepend(&mut self, item: T) {
        self.insert(0, item);
    }

    pub(crate) fn remove_item(&mut self, item: &T) {
        if let Some(index) = self.index_of(item) {
            self.remove(index);
        }
    }

    fn index_of(&self, item: &T) -> Option<usize> {
        match self {
            ListContent::Small(v) => v.iter().position(|r| r == item),
            ListContent::Large(v) => v.index_of(item),
        }
    }

    fn size(&self) -> usize {
        match self {
            ListContent::Small(v) => v.len(),
            ListContent::Large(v) => v.size(),
        }
    }

    fn contains(&self, item: &T) -> bool {
        match self {
            ListContent::Small(v) => v.iter().position(|r| r == item).map_or(false, |v| true),
            ListContent::Large(v) => v.contains(item),
        }
    }

    // iterate over all list items
    pub(crate) fn iter(&self) -> ListContentIter<'_, T> {
        match self {
            ListContent::Small(v) => ListContentIter::Small(v.iter()),
            ListContent::Large(v) => ListContentIter::Large(v.iter_from(0).unwrap()),
        }
    }
}

enum ListContentIter<'a, T: Debug> {
    Small(std::slice::Iter<'a, T>),
    Large(ValueIter<'a, T>),
}

impl<'a, V: Debug> Iterator for ListContentIter<'a, V> {
    type Item = (&'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ListContentIter::Small(v) => v.next(),
            ListContentIter::Large(v) => v.next(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct InnerList {
    pub(crate) store: WeakStoreRef,  // 16
    pub(crate) flags: u8,            // 1
    pub(crate) data: AItem,          // 16
    pub(crate) parent: Option<Type>, // 16
    pub(crate) content: ListContent<Type>,
}

#[derive(Clone, Debug)]
pub struct NList {
    inner: Rc<RefCell<InnerList>>,
}

impl NList {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        let data = AItem::new(id).with_kind(ItemKind::List);
        Self {
            inner: Rc::new(RefCell::new(InnerList {
                store,
                flags: 0,
                data,
                parent: None,
                content: ListContent::default(),
            })),
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.inner.borrow().content.size()
    }

    #[inline]
    pub fn get(&self, key: impl Into<ItemKey>) -> Option<Type> {
        if let ItemKey::Number(offset) = key.into() {
            return self.inner.borrow().content.get(offset).map(|v| v.clone());
        }
        None
    }

    #[inline]
    pub fn content(&self) -> Content {
        // let types = self.borrow().content.iter()
        Content::Types(vec![])
    }

    #[inline]
    pub(crate) fn index_of(&self, target: &Type) -> Option<usize> {
        self.inner.borrow().content.index_of(target)
    }

    #[inline]
    pub(crate) fn prepend(&self, item: impl Into<Type>) {
        let item = item.into();
        item.set_parent(Some(self.into()));

        let content = &mut self.borrow_mut().content;

        // set the right id
        content.first().map(|f| item.set_right_id(f.id()));
        content.prepend(item);
    }

    #[inline]
    /// append an item to the end of the list
    pub fn append(&self, item: impl Into<Type>) {
        let item = item.into();
        item.set_parent(Some(self.into()));

        let content = &mut self.borrow_mut().content;
        // set the left id
        content.last().map(|f| item.set_left_id(f.id()));
        content.prepend(item);
    }

    pub fn insert(&self, offset: usize, item: impl Into<Type>) {
        let size = self.size();
        let item = item.into();

        if offset == 0 {
            self.prepend(item);
        } else if offset >= size {
            self.append(item);
        } else {
            self.borrow_mut().content.insert(offset, item);
        }
    }

    #[inline]
    fn remove(&self, key: ItemKey) {
        if let ItemKey::Number(offset) = key {
            if offset < self.size() {
                self.borrow_mut().content.remove(offset);
            }
        }
    }

    // remove the item front the list items
    #[inline]
    pub(crate) fn remove_child(&self, child: &Type) {
        self.borrow_mut().content.remove_item(child);
        // child.disconnect();
    }

    // move the item after the target item
    pub(crate) fn move_after(&self, before: &Type, target: &Type) {
        if let Some(index) = self.borrow().content.index_of(before) {
            self.move_to(index + 1, target);
        }
    }

    // move the item before the target item
    pub(crate) fn move_before(&self, after: &Type, target: &Type) {
        if let Some(index) = self.borrow().content.index_of(after) {
            self.move_to(index, target);
        }
    }

    // move the item to the offset position in the new parent list
    pub(crate) fn move_to(&self, offset: usize, target: &Type) {
        if creates_cycle(&self.into(), target) {
            warn!("can not move nodes within, creates cycle");
            return;
        }

        let store = self.borrow().store.clone();
        let id = store.upgrade().unwrap().borrow_mut().next_id();
        let mover: Type = NMove::new(id, target.clone(), store.clone()).into();

        // FIXME: use target.mark_moved
        // target.item_ref().mark_moved();

        store
            .upgrade()
            .unwrap()
            .borrow_mut()
            .add_mover(target.id(), mover.clone());

        self.insert(offset, mover);
    }

    // remove all children,
    // internally all children are marked deleted
    #[inline]
    pub(crate) fn clear(&self) {
        self.borrow_mut().content.iter().for_each(|t| t.delete())
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let content = self
            .borrow()
            .content
            .iter()
            .filter(|t| t.is_visible())
            .map(|t| t.to_json())
            .collect();

        serde_json::Value::Array(content)
    }
}

impl Deref for NList {
    type Target = Rc<RefCell<InnerList>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl WithIdRange for NList {
    #[inline]
    fn range(&self) -> IdRange {
        self.borrow().id().range(1)
    }
}

impl Encode for NList {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        // self.inner.borrow().data.encode(e, ctx);
    }
}

impl Serialize for NList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("List", 1 + 1)?;
        // self.serialize_with(&mut s)?;

        // let content = self
        //     .visible_item_iter()
        //     .map(|item| serde_json::to_value(item).unwrap_or_default())
        //     .collect::<Vec<_>>();

        // s.serialize_field("content", &serde_json::Value::Array(content))?;

        s.end()
    }
}

#[cfg(test)]
mod test {
    use crate::doc::Doc;

    #[test]
    fn test_nlist() {
        let doc = &Doc::default();

        let list = &doc.list();
        doc.set("list", list.clone());
        assert_eq!(list.size(), 0);

        ["a", "b", "c"]
            .iter()
            .map(|s| doc.atom(*s))
            .for_each(|atom| list.append(atom));

        assert_eq!(list.size(), 3);

        let yaml = serde_yaml::to_string(&list).unwrap();

        println!("{}", yaml);

        let expect = r#"id: (0, 2)
kind: list
parent_id: (0, 1)
content:
- content: a
  id: (0, 3)
  kind: atom
  parent_id: (0, 2)
- content: b
  id: (0, 4)
  kind: atom
  left_id: (0, 3)
  parent_id: (0, 2)
- content: c
  id: (0, 5)
  kind: atom
  left_id: (0, 4)
  parent_id: (0, 2)
"#;

        assert_eq!(yaml, expect);
    }

    #[test]
    fn test_list_of_list() {
        let doc = &Doc::default();

        let list1 = &doc.list();
        doc.set("list", list1.clone());
        assert_eq!(list1.size(), 0);

        let list2 = &doc.list();
        list1.append(list2.clone());

        assert_eq!(list1.size(), 1);

        let atom = doc.atom("a");
        list2.append(atom.clone());

        assert_eq!(list2.size(), 1);

        let atom = doc.atom("b");
        list1.append(atom.clone());

        assert_eq!(list1.size(), 2);

        let root = doc.root.clone();
        let yaml = serde_yaml::to_string(&root).unwrap();
        // println!("{}", yaml);

        let expect = r#"id: (0, 1)
kind: map
content:
  list:
    content:
    - content:
      - content: a
        id: (0, 4)
        kind: atom
        parent_id: (0, 3)
      id: (0, 3)
      kind: list
      parent_id: (0, 2)
    - content: b
      id: (0, 5)
      kind: atom
      left_id: (0, 3)
      parent_id: (0, 2)
    id: (0, 2)
    kind: list
    parent_id: (0, 1)
"#;
        assert_eq!(yaml, expect);

        // println!("{}", serde_yaml::to_string(doc).unwrap());
    }
}
