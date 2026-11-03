use hashbrown::HashMap;
use std::ops::Deref;

use serde::ser::SerializeStruct;
use serde::Serialize;

use crate::id::{Id, IdRange, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemIterator, ItemKey, ItemKind, ItemRef, Linked, StartEnd};
use crate::mark::{Mark, MarkContent};
use crate::nmark::NMark;
use crate::store::WeakStoreRef;
use crate::types::Type;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct NMap {
    pub(crate) item: ItemRef,
}

impl NMap {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        let data = ItemData {
            kind: ItemKind::Map,
            id,
            ..ItemData::default()
        };

        let item_ref = ItemRef::new(data.into(), store);

        Self { item: item_ref }
    }

    /// size of the map
    pub(crate) fn size(&self) -> u32 {
        let item = self.borrow();
        let map = item.as_map(&self.store);
        map.len() as u32
    }

    /// item field value used in kv entry as key
    fn field(&self) -> Option<String> {
        self.borrow().field(&self.item_ref().store)
    }

    pub(crate) fn content(&self) -> Content {
        let types = self.borrow().as_list();
        Content::Types(types)
    }

    pub(crate) fn set_content(&self, content: impl Into<Content>) {
        self.item_ref().set_content(content.into());
    }

    pub(crate) fn item_ref(&self) -> ItemRef {
        self.item.clone()
    }
}

impl Deref for NMap {
    type Target = ItemRef;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl NMap {
    pub(crate) fn add_mark(&self, mark: impl Into<Mark>) {
        let content = MarkContent::new(self.id().into(), mark.into());
        let id = self
            .store
            .upgrade()
            .unwrap()
            .borrow_mut()
            .next_id_range(1)
            .id();

        let mark = NMark::new(id, Content::Mark(content), self.store.clone());

        // self.item_ref().add_mark(mark);
    }

    pub(crate) fn get(&self, key: impl Into<ItemKey>) -> Option<Type> {
        let item = self.borrow();
        let key = key.into().as_string();
        let map = self.visible_children();
        let item = map.get(&key);

        item.cloned()
    }

    pub(crate) fn set(&self, field: impl Into<String>, item: impl Into<Type>) {
        let item = item.into();
        let item_ref = item.item_ref();
        let store = item_ref.store.upgrade().unwrap();
        let field_id = store.borrow_mut().get_field_id(&field.into());
        item.set_parent(Some(self.into()));
        item_ref.borrow_mut().data.field = Some(field_id);
        self.item_ref().append(item);
    }

    pub(crate) fn remove(&self, key: ItemKey) {
        let map = self.visible_children();
        let value = map.get(&key.as_string());
        if let Some(value) = value {
            value.delete();
        }
    }

    pub(crate) fn remove_child(&self, child: &Type) {
        child.item_ref().disconnect();
    }

    //
    pub(crate) fn keys(&self) -> Vec<String> {
        self.visible_children().keys().cloned().collect()
    }

    pub(crate) fn values(&self) -> Vec<Type> {
        self.visible_children().values().cloned().collect()
    }

    pub(crate) fn clear(&self) {
        let item = self.borrow();
        let map = item.as_map(&self.store);
        for item in map.values() {
            item.delete();
        }
    }

    fn visible_children(&self) -> HashMap<String, Type> {
        let mut curr = self.start();
        let mut map = HashMap::new();
        while let Some(item) = curr {
            if item.is_visible() {
                if let Some(field) = item.field() {
                    map.insert(field, Type::from(item.clone()));
                }
            }

            curr = item.right();
        }

        map
    }

    #[inline]
    pub(crate) fn delete(&self) {
        self.item.delete(1);
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let mut json = self.borrow().to_json();

        let map = self.visible_children();
        let mut content = serde_json::Map::new();
        for (key, value) in map.iter() {
            content.insert(key.clone(), value.to_json());
        }

        serde_json::Value::Object(content)
    }
}

impl Serialize for NMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("Doc", self.borrow().serialize_size() + 1)?;
        self.serialize_with(&mut s)?;

        // let map = self.borrow().as_map(self.store.clone());
        // let content = serde_json::to_value(map).unwrap_or_default();

        let mut map = HashMap::new();
        self.item_iter().for_each(|item| {
            let field = item.field().unwrap_or_default();
            if (item.is_visible()) {
                let value = serde_json::to_value(item).unwrap_or_default();
                map.insert(field, value);
            } else {
                map.remove(&field);
            }
        });
        let content = serde_json::to_value(map).unwrap_or_default();

        s.serialize_field("content", &content)?;

        s.end()
    }
}

impl WithId for NMap {
    #[inline]
    fn id(&self) -> Id {
        self.item.borrow().id()
    }
}

impl WithIdRange for NMap {
    fn range(&self) -> IdRange {
        self.item.borrow().id().range(1)
    }
}

impl From<ItemRef> for NMap {
    fn from(item: ItemRef) -> Self {
        Self { item }
    }
}

#[cfg(test)]
mod tests {
    use crate::doc::Doc;
    use crate::print_yaml;
    use serde_json::json;

    #[test]
    fn test_map() {
        let doc = Doc::default();
        let map = doc.map();
        doc.set("map", map.clone());

        let map = doc.get("map").unwrap();
        assert_eq!(map.size(), 0);

        //         let atom = doc.atom("a");
        //         map.set("a", atom.clone());
        //
        //         let atom = doc.atom("b");
        //         map.set("b", atom.clone());
        //
        //         let yaml = serde_yaml::to_string(&map).unwrap();
        //         println!("{}", yaml);
        //
        //         let expect = r#"id: (1, 1)
        // kind: map
        // parent_id: (0, 1)
        // content:
        //   a:
        //     content: a
        //     id: (1, 2)
        //     kind: atom
        //     parent_id: (1, 1)
        //   b:
        //     content: b
        //     id: (1, 3)
        //     kind: atom
        //     left_id: (1, 2)
        //     parent_id: (1, 1)
        // "#;
        //         assert_eq!(yaml, expect);
    }

    #[test]
    fn test_node_1() {
        let doc = Doc::default();
        let map = doc.map();
        doc.set("content", map.clone());

        // map.set("kind", doc.atom("node"));
        // map.set("id", doc.atom("1"));
        map.set("name", doc.atom("page"));

        let list = doc.list();
        map.set("children", list.clone());

        let p1 = doc.map();
        // p1.set("id", doc.atom("2"));
        // p1.set("kind", doc.atom("node"));
        p1.set("name", doc.atom("paragraph"));

        let t1 = doc.text();
        t1.insert(0, doc.string("Hello, world!"));
        p1.set("children", t1.clone());

        let p2 = doc.map();
        p2.set("id", doc.atom("3"));
        p2.set("kind", doc.atom("node"));
        p2.set("name", doc.atom("paragraph"));

        assert_eq!(p2.get("id").unwrap().text_content(), "3");
        assert_eq!(p2.get("kind").unwrap().text_content(), "node");
        assert_eq!(p2.get("name").unwrap().text_content(), "paragraph");
        assert_eq!(
            p2.to_json(),
            json!({"id":"3","kind":"node","name":"paragraph"})
        );

        let t2 = doc.text();
        t2.insert(0, doc.string("Goodbye, world!"));
        p2.set("children", t2.clone());

        list.append(p1.clone());
        list.append(p2.clone());
        // let yaml = serde_json::to_string(&doc).unwrap();
        // println!("---\n{}", yaml);

        assert_eq!(list.size(), 2);

        let json = doc.to_json();
        // println!("---\n{}", serde_json::to_string_pretty(&json).unwrap());
        // print_yaml(&doc);
    }
}
