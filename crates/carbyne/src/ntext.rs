use std::io::BufRead;
use std::ops::Deref;

use serde::ser::SerializeStruct;
use serde::Serialize;

use crate::id::{Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemIterator, ItemKind, ItemRef};
use crate::store::WeakStoreRef;
use crate::types::Type;

#[derive(Clone, Debug)]
pub struct NText {
    pub(crate) item: ItemRef,
}

impl NText {
    pub(crate) fn on_insert(&self, p0: &Type) {}
}

impl NText {
    pub(crate) fn slice(&self, start: u32, end: u32) -> Vec<Type> {
        let start = self.find_at_offset(start);
        let end = self.find_at_offset(end);

        let store = self.store.upgrade().unwrap();
        if let Some(item) = &end.0 {
            let items = item.split(end.1);
            item.replace(items);
        }

        if let Some(item) = &start.0 {
            let items = item.split(start.1);
            item.replace(items);
        }

        Vec::new()
    }
}

impl NText {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        let data = ItemData {
            id,
            kind: ItemKind::Text,
            ..ItemData::default()
        };

        Self {
            item: ItemRef::new(data.into(), store),
        }
    }

    pub(crate) fn clear(&self) {
        self.item_ref()
            .borrow()
            .items()
            .iter()
            .for_each(|item| item.delete());
    }

    pub(crate) fn content(&self) -> Content {
        let items = self.borrow().as_list();
        Content::Types(items)
    }

    pub(crate) fn size(&self) -> u32 {
        self.visible_item_iter()
            .fold(0, |acc, item| acc + item.size())
    }

    pub fn append(&self, item: impl Into<Type>) {
        let item = item.into();
        assert!(item.kind().is_string());
        self.item.append(item.clone());
        item.set_parent(Some(self.into()));
    }

    pub fn prepend(&self, item: impl Into<Type>) {
        let item = item.into();
        assert!(item.kind().is_string());
        self.item.prepend(item);
    }

    /// Insert string in text
    pub fn insert(&self, offset: u32, item: impl Into<Type>) {
        let item = item.into();

        assert!(item.kind().is_string());

        if offset == 0 {
            self.prepend(item);
        } else if offset >= self.size() {
            self.append(item);
        } else {
            // find the target item offset
            let (target, offset) = self.find_at_offset(offset);

            if let Some(target) = target {
                if offset == 0 {
                    target.insert_before(item);
                } else if offset >= target.size() {
                    target.insert_after(item);
                } else {
                    let items = target.split(offset);
                    items.0.insert_after(item);
                }
            }
        }
    }

    // find item string child at offset
    fn find_at_offset(&self, offset: u32) -> (Option<Type>, u32) {
        let items = self.borrow().as_list();
        let mut target_offset = 0;
        let mut target = None;

        if offset == 0 {
            return (items.first().cloned(), 0);
        }

        for item in self.item.visible_item_iter() {
            let size = item.size();
            if target_offset + size > offset {
                target = Some(item);
                break;
            }
            target_offset += size;
        }

        if let Some(target) = &target {
            (Some(target.into()), offset - target_offset)
        } else {
            (None, target_offset)
        }
    }

    pub(crate) fn item_ref(&self) -> ItemRef {
        self.item.clone()
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let items = self.borrow().as_list();
        let items: Vec<_> = items.iter().map(|item| item.to_json()).collect();

        items.into()
    }

    // raw un marked text content
    pub(crate) fn text_content(&self) -> String {
        self.visible_item_iter()
            .map(|item| item.text_content())
            .collect()
    }
}

impl Serialize for NText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("Text", self.borrow().serialize_size() + 1)?;
        self.borrow().serialize_with(&mut s)?;

        let content = self
            .visible_item_iter()
            .map(|item| serde_json::to_value(item).unwrap_or_default())
            .collect::<Vec<_>>();

        s.serialize_field("content", &serde_json::Value::Array(content))?;

        s.end()
    }
}

impl WithId for NText {
    #[inline]
    fn id(&self) -> Id {
        self.item.borrow().id()
    }
}

impl WithIdRange for NText {
    fn range(&self) -> IdRange {
        self.borrow().id().range(1)
    }
}

impl Deref for NText {
    type Target = ItemRef;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl From<ItemRef> for NText {
    fn from(item: ItemRef) -> Self {
        Self { item }
    }
}

#[cfg(test)]
mod tests {
    use crate::doc::Doc;

    #[test]
    fn test_text() {
        let doc = Doc::default();
        let text = doc.text();
        doc.set("text", text.clone());

        assert_eq!(text.size(), 0);

        let s1 = doc.string("hello");
        text.append(s1.clone());

        let s2 = doc.string("world");
        text.prepend(s2.clone());

        let yaml = serde_yaml::to_string(&text).unwrap();
        println!("{}", yaml);
    }

    #[test]
    fn test_insert_between_string() {
        let doc = Doc::default();
        let text = doc.text();
        doc.set("text", text.clone());

        let s1 = doc.string("hello");
        text.append(s1.clone());

        let s2 = doc.string("world");
        text.prepend(s2.clone());

        let s3 = doc.string("foo");
        text.insert(3, s3.clone());

        // let yaml = serde_yaml::to_string(&text).unwrap();
        // println!("{}", yaml);

        assert_eq!(text.item_ref().borrow().items().len(), 4);

        let text_ref = &text.item_ref();
        let text_ref = text_ref.borrow();
        let items = text_ref.items().clone();
        let first = items.first().unwrap();
        assert_eq!(
            items.first().unwrap().content().to_json(),
            "wor".to_string()
        );
        assert_eq!(items.get(1).unwrap().content().to_json(), "foo".to_string());
        assert_eq!(items.get(2).unwrap().content().to_json(), "ld".to_string());
        assert_eq!(
            items.get(3).unwrap().content().to_json(),
            "hello".to_string()
        );
    }
}
