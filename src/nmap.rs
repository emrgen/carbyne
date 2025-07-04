use crate::bimapid::FieldId;
use crate::id::{Id, IdRange, WithId, WithIdRange};
use crate::inner_type::InnerType;
use crate::item::{Content, ItemKey, ItemKind};
use crate::mark::Mark;
use crate::nlist::NList;
use crate::nmove::NMove;
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::Item;
use hashbrown::HashMap;
use serde::ser::SerializeStruct;
use serde::Serialize;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct MapRef(Rc<RefCell<NMap>>);

impl Hash for MapRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

#[derive(Debug, Default)]
struct MapContent {
    map: HashMap<FieldId, Vec<Type>>,
}

#[derive(Debug)]
pub(crate) struct NMap {
    pub(crate) store: WeakStoreRef,  // 16
    pub(crate) flags: u8,            // 1
    pub(crate) item: Item,           // 16
    pub(crate) parent: Option<Type>, // 16
    content: MapContent,             // 16
}

impl Eq for NMap {}

impl PartialEq for NMap {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl NMap {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        Self {
            store,
            flags: 0,
            item: Item::new(id),
            parent: None,
            content: MapContent::default(),
        }
    }

    /// size of the map
    pub(crate) fn size(&self) -> usize {
        self.children().len()
    }

    // pub(crate) fn content(&self) -> Content {
    //     let types = self
    //         .content
    //         .map
    //         .iter()
    //         .filter(|(_, t)| t.last().map_or(false, |t| t.is_visible()))
    //         .map(|(f, t)| t.last().cloned().unwrap())
    //         .collect();
    //
    //     Content::Types(types)
    // }

    pub(crate) fn add_mark(&self, mark: impl Into<Mark>) {
        // let content = MarkContent::new(self.id().into(), mark.into());
        // let id = self
        //     .store
        //     .upgrade()
        //     .unwrap()
        //     .borrow_mut()
        //     .next_id_range(1)
        //     .id();
        //
        // let mark = NMark::new(id, Content::Mark(content), self.store.clone());

        // self.item_ref().add_mark(mark);
    }

    pub(crate) fn get(&self, key: impl Into<ItemKey>) -> Option<Type> {
        let key = key.into().as_string();
        if let Some(field_id) = self.map_field_id(&key) {
            return self
                .content
                .map
                .get(&field_id)
                .map(|v| v.last().cloned())
                .flatten();
        }

        None
    }

    fn map_field_id(&self, key: &String) -> Option<FieldId> {
        self.store
            .upgrade()
            .map(|store| store.borrow().fields.get_field_id(key).cloned())
            .flatten()
    }

    pub(crate) fn set(&self, field: impl Into<String>, item: impl Into<Type>) {
        let item = item.into();
        if let Some(field_id) = self.map_field_id(&field.into()) {
            // insert into inner content by field id
        }
    }

    pub(crate) fn remove(&self, key: ItemKey) {
        let field = key.as_string();
        if let Some(field_id) = self.map_field_id(&field) {
            self.content
                .map
                .get(&field_id)
                .map(|v| v.last().map(|t| t.delete()));
        }
    }

    pub(crate) fn remove_child(&self, child: &Type) {
        // self.borrow_mut().content.remove_item(child);
        child.disconnect();
    }

    //
    pub(crate) fn keys(&self) -> Vec<String> {
        let items: Vec<_> = self.children();
        let store = self.store.upgrade().unwrap();
        let fields = &store.borrow().fields;
        let mut keys = Vec::new();
        for t in items {
            if let Some(field) = t.field(fields) {
                keys.push(field.clone())
            }
        }

        keys
    }

    pub(crate) fn values(&self) -> Vec<Type> {
        self.children()
    }

    // delete all visible last items for each key
    pub(crate) fn clear(&self) {
        self.content
            .map
            .iter()
            .filter(|(_, t)| t.last().map_or(false, |t| t.is_visible()))
            .for_each(|(_, t)| {
                if let Some(last) = t.last() {
                    last.delete();
                }
            });
    }

    fn children(&self) -> Vec<Type> {
        let mut list = Vec::new();
        self.content
            .map
            .iter()
            .filter(|(_, t)| t.last().map_or(false, |t| t.is_visible()))
            .for_each(|(_, t)| {
                if let Some(last) = t.last() {
                    list.push(last.clone());
                }
            });

        list
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let store = self.store.upgrade().unwrap();
        let fields = &store.borrow().fields;
        let map = self.children();
        let mut content = serde_json::Map::new();
        for t in self.children().iter() {
            content.insert(t.field(fields).unwrap(), t.to_json());
        }

        serde_json::Value::Object(content)
    }
}

impl InnerType for NMap {
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

impl Deref for NMap {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl DerefMut for NMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl WithIdRange for NMap {
    fn range(&self) -> IdRange {
        self.id.range(1)
    }
}

impl Serialize for NMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("Map", 1)?;
        // self.serialize_with(&mut s)?;

        // let map = self.borrow().as_map(self.store.clone());
        // let content = serde_json::to_value(map).unwrap_or_default();

        let store = self.store.upgrade().unwrap();
        let fields = &store.borrow().fields;

        let mut map = HashMap::new();
        self.children().iter().for_each(|item| {
            let field = item.field(fields).unwrap_or_default();
            if item.is_visible() {
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

impl Hash for NMap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::doc::Doc;
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

        // list.append(p1.clone());
        // list.append(p2.clone());
        // let yaml = serde_json::to_string(&doc).unwrap();
        // println!("---\n{}", yaml);

        assert_eq!(list.size(), 2);

        let json = doc.to_json();
        // println!("---\n{}", serde_json::to_string_pretty(&json).unwrap());
        // print_yaml(&doc);
    }
}
