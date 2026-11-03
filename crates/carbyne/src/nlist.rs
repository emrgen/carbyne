use crate::cycle::creates_cycle;
use crate::id::{Id, IdRange, WithId, WithIdRange, WithTarget};
use crate::index::{BTreeIndex, IBTree, ItemIndexMap};
use crate::item::{
    ContainerKind, Content, ItemData, ItemIterator, ItemKey, ItemKind, ItemRef, Linked, StartEnd,
    WithIndex,
};
use crate::nmove::NMove;
use crate::store::WeakStoreRef;
use crate::types::Type;
use log::warn;
use serde::ser::{Serialize, SerializeStruct};
use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct NList {
    item: ItemRef,
    // TODO: dynamically upgrade/downgrade the list to binary-tree/btree when the nlist is too large
    list: Rc<RefCell<IBTree>>,
}

impl NList {
    pub(crate) fn remove_child(&self, child: &Type) {
        self.list.borrow_mut().remove(child);
        child.item_ref().disconnect();
    }

    /// move the item after the target item
    pub(crate) fn move_after(&self, before: &Type, target: &Type) {
        let index = self.list.borrow().index_of(before);
        if index < 0 || index >= (self.size() as i32) {
            println!("move_after: ref item {} not found", before.id());
            return;
        }

        self.move_to((index + 1) as u32, target);
    }

    /// move the item before the target item
    pub(crate) fn move_before(&self, after: &Type, target: &Type) {
        let index = self.list.borrow().index_of(after);
        if index < 0 || index >= self.size() as i32 {
            println!("move_before: ref item {} not found", after.id());
            return;
        }

        self.move_to(index as u32, target);
    }

    /// move the item to the offset position in the new parent list
    pub(crate) fn move_to(&self, offset: u32, target: &Type) {
        if creates_cycle(&self.into(), target) {
            warn!("can not move nodes within, creates cycle");
            return;
        }

        let id = self.store.upgrade().unwrap().borrow_mut().next_id();
        let mover: Type = NMove::new(id, target.clone(), self.store.clone()).into();

        target.item_ref().mark_moved();

        self.store
            .upgrade()
            .unwrap()
            .borrow_mut()
            .add_mover(target.id(), mover.clone());

        self.insert(offset, mover);
    }
}

impl NList {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        let data = ItemData {
            id,
            kind: ItemKind::List,
            ..ItemData::default()
        };

        Self {
            item: ItemRef::new(data.into(), store),
            list: Rc::new(RefCell::new(Default::default())),
        }
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.list.borrow().size() as u32
    }

    #[inline]
    pub fn get(&self, key: impl Into<ItemKey>) -> Option<Type> {
        if let ItemKey::Number(offset) = key.into() {
            return self.list.borrow().at_index(offset).map(|v| v.clone());
        }
        None
    }

    #[inline]
    pub(crate) fn field(&self) -> Option<String> {
        self.borrow().field(&self.item_ref().store)
    }

    #[inline]
    pub fn content(&self) -> Content {
        let types = self.borrow().as_list();
        Content::Types(types)
    }

    #[inline]
    pub(crate) fn item_ref(&self) -> ItemRef {
        self.item.clone()
    }

    #[inline]
    pub(crate) fn index_of(&self, target: &Type) -> i32 {
        self.list.borrow().index_of(target)
    }
}

impl Deref for NList {
    type Target = ItemRef;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl NList {
    fn prepend(&self, item: impl Into<Type>) {
        let item = item.into();
        #[cfg(feature = "fugue")]
        {
            self.item.prepend(item.clone());
            Type::add_frac_index(&item);
            self.on_insert(&item);
        }
        #[cfg(not(feature = "fugue"))]
        {
            item.set_parent(Some(self.into()));
            self.item.prepend(item.clone());
            Type::add_frac_index(&item);
            self.on_insert(&item);
        }
    }

    /// append an item to the end of the list
    pub fn append(&self, item: impl Into<Type>) {
        let item = item.into();
        item.set_parent(Some(self.into()));
        self.item.append(item.clone());
        Type::add_frac_index(&item);
        self.on_insert(&item);
    }

    pub fn insert(&self, offset: u32, item: impl Into<Type>) {
        let size = self.list.borrow().size();
        let item = item.into();
        // item.set_container(self.item.clone());

        if offset == 0 {
            self.prepend(item);
        } else if offset >= size as u32 {
            self.append(item);
        } else {
            let next = {
                let list = self.list.borrow();

                // quickly find the item at the offset index using the binary search
                list.at_index(offset).cloned()
            };

            if let Some(next) = next {
                next.insert_before(item);
            } else {
                self.append(item);
            }
        }
    }

    fn fugue_append(&self, offset: u32, item: impl Into<Type>) {}
    fn fugue_prepend(&self, offset: u32, item: impl Into<Type>) {}
    fn fugue_insert(&self, offset: u32, item: impl Into<Type>) {}

    fn remove(&self, key: ItemKey) {
        if let ItemKey::Number(offset) = key {
            if offset < self.size() {
                let items = self.borrow().as_list();
                let item = items[offset as usize].clone();
                item.delete();
            }
        }
    }

    #[inline]
    pub(crate) fn delete(&self) {
        self.item.delete(1);
    }

    #[inline]
    pub(crate) fn clear(&self) {
        let items = self.borrow().as_list();
        for item in items {
            item.delete();
        }
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = self.borrow().to_json();
        let items = self.borrow().as_list();

        let content = items.iter().map(|item| item.to_json()).collect();

        serde_json::Value::Array(content)
    }
}

impl NList {
    #[inline]
    pub(crate) fn on_insert(&self, child: &Type) {
        self.list.borrow_mut().insert(child.clone());
    }
}

impl Serialize for NList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("List", self.borrow().serialize_size() + 1)?;
        self.serialize_with(&mut s)?;

        let content = self
            .visible_item_iter()
            .map(|item| serde_json::to_value(item).unwrap_or_default())
            .collect::<Vec<_>>();

        s.serialize_field("content", &serde_json::Value::Array(content))?;

        s.end()
    }
}

impl WithId for NList {
    #[inline]
    fn id(&self) -> Id {
        self.item.borrow().id()
    }
}

impl WithIdRange for NList {
    #[inline]
    fn range(&self) -> IdRange {
        self.borrow().id().range(1)
    }
}

impl From<ItemRef> for NList {
    #[inline]
    fn from(item: ItemRef) -> Self {
        Self {
            item,
            ..Default::default()
        }
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
