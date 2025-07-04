use crate::bimapid::FieldId;
use crate::cycle::creates_cycle;
use crate::delete::DeleteItem;
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, WithId, WithIdRange, WithTarget};
use crate::index::{BTreeIndex, IBTree, ItemIndexMap};
use crate::index_tree::{IndexTree, ValueIter};
use crate::inner_type::InnerType;
use crate::item::{Content, ItemData, ItemKey, ItemKind};
use crate::list_content::ListContent;
use crate::natom::NAtom;
use crate::nmove::{MoveRef, NMove};
use crate::store::{DocStore, WeakStoreRef};
use crate::types::Type;
use crate::{index_tree, Item};
use log::warn;
use serde::ser::{Serialize, SerializeStruct};
use std::cell::{RefCell, RefMut};
use std::collections::vec_deque::Iter;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut, Index};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct ListRef(Rc<RefCell<NList>>);

impl ListRef {
    fn append(&self, child: Type) {
        self.0.borrow_mut().append(child);
    }

    fn prepend(&self, child: Type) {
        self.0.borrow_mut().prepend(child);
    }

    fn insert(&self, offset: usize, child: Type) {
        self.0.borrow_mut().insert(offset, child);
    }

    pub(crate) fn children(&self) -> Vec<Type> {
        self.0.borrow().children()
    }
}

impl From<NList> for ListRef {
    fn from(value: NList) -> Self {
        ListRef(Rc::new(RefCell::new(value)))
    }
}

impl Hash for ListRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

#[derive(Debug)]
pub(crate) struct NList {
    pub(crate) store: WeakStoreRef,        // 16
    pub(crate) flags: u8,                  // 1
    pub(crate) item: Item,                 // ??
    pub(crate) parent: Option<Type>,       // 16
    pub(crate) content: ListContent<Type>, // 16
}

impl NList {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        Self {
            store,
            flags: 0,
            item: Item::new(id),
            parent: None,
            content: ListContent::default(),
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.content.size()
    }

    #[inline]
    pub fn get(&self, key: impl Into<ItemKey>) -> Option<Type> {
        if let ItemKey::Number(offset) = key.into() {
            return self.content.get(offset).map(|v| v.clone());
        }
        None
    }

    #[inline]
    pub(crate) fn index_of(&self, target: &Type) -> Option<usize> {
        self.content.index_of(target)
    }

    #[inline]
    fn prepend(&mut self, item: Type) {
        self.content.first().map(|f| item.set_right_id(f.id()));
        self.content.prepend(item);
    }

    #[inline]
    /// append an item to the end of the list
    fn append(&mut self, item: Type) {
        self.content.last().map(|f| item.set_left_id(f.id()));
        self.content.prepend(item);
    }

    fn insert(&mut self, offset: usize, item: Type) {
        let size = self.size();

        if offset == 0 {
            self.prepend(item);
        } else if offset >= size {
            self.append(item);
        } else {
            self.content.insert(offset, item);
        }
    }

    #[inline]
    fn remove(&self, key: ItemKey) {
        if let ItemKey::Number(offset) = key {
            if offset < self.size() {
                self.content.remove(offset);
            }
        }
    }

    // remove the item front the list items
    #[inline]
    pub(crate) fn remove_child(&mut self, child: &Type) {
        self.content.remove_item(child);
        // child.disconnect();
    }

    // move the item after the target item
    pub(crate) fn move_after(&mut self, before: &Type, target: &Type) {
        if let Some(index) = self.content.index_of(before) {
            self.move_to(index + 1, target);
        }
    }

    // move the item before the target item
    pub(crate) fn move_before(&mut self, after: &Type, target: &Type) {
        if let Some(index) = self.content.index_of(after) {
            self.move_to(index, target);
        }
    }

    // move the item to the offset position in the new parent list
    pub(crate) fn move_to(&mut self, offset: usize, target: &Type) {
        // if creates_cycle(&self.into(), target) {
        //     warn!("can not move nodes within, creates cycle");
        //     return;
        // }

        let store = self.store.clone();
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
        self.content.iter().for_each(|t| t.delete())
    }

    pub(crate) fn children(&self) -> Vec<Type> {
        self.content
            .iter()
            .filter(|t| t.is_visible())
            .map(|t| t.clone())
            .collect()
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let content = self
            .content
            .iter()
            .filter(|t| t.is_visible())
            .map(|t| t.to_json())
            .collect();

        serde_json::Value::Array(content)
    }
}

impl InnerType for NList {
    fn store(&self) -> &WeakStoreRef {
        &self.store
    }

    fn flag(&self) -> &u8 {
        &self.flags
    }

    fn flag_mut(&mut self) -> &mut u8 {
        &mut self.flags
    }

    fn size(&self) -> usize {
        1
    }

    fn parent(&self) -> &Option<Type> {
        &self.parent
    }
}

impl Deref for NList {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl DerefMut for NList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl WithIdRange for NList {
    #[inline]
    fn range(&self) -> IdRange {
        self.id.range(1)
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

impl Hash for NList {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
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
