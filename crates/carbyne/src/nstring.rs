use std::ops::Deref;

use serde::ser::SerializeStruct;
use serde::Serialize;
use serde_json::Value;

use crate::id::{ClockTick, Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind, ItemRef};
use crate::mark::{Mark, MarkContent};
use crate::nmark::NMark;
use crate::store::WeakStoreRef;
use crate::types::Type;

#[derive(Clone, Debug)]
pub struct NString {
    pub(crate) item: ItemRef,
}

impl NString {
    pub(crate) fn new(id: Id, string: String, store: WeakStoreRef) -> Self {
        let data = ItemData {
            id,
            kind: ItemKind::String,
            content: Content::String(string),
            ..ItemData::default()
        };

        Self {
            item: ItemRef::new(data.into(), store),
        }
    }

    #[inline]
    pub(crate) fn content(&self) -> Content {
        self.borrow().content()
    }

    #[inline]
    pub(crate) fn size(&self) -> u32 {
        match self.borrow().content {
            Content::String(ref s) => s.len() as u32,
            _ => panic!("NString has invalid content"),
        }
    }

    // delete string
    #[inline]
    pub(crate) fn delete(&self) {
        self.item.delete(self.size());
    }

    // add mark to string
    pub(crate) fn add_mark(&self, mark: Mark) {
        // create mark content, which includes the string id range
        // e.g. the range of the string "hello" is 0..5 the mark content will be (0..5, mark)
        let content = MarkContent::new(self.id().range(self.size()), mark.clone());
        // get next id within the range of the string
        let id = self
            .store
            .upgrade()
            .unwrap()
            .borrow_mut()
            .next_id_range(self.size() as ClockTick)
            .id();

        let mark = NMark::new(id, Content::Mark(content), self.store.clone());

        // self.item_ref().add_mark(mark);
    }

    #[inline]
    pub(crate) fn item_ref(&self) -> ItemRef {
        self.item.clone()
    }

    pub(crate) fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();

        map.insert("text".to_string(), self.borrow().content().to_json());

        map.into()
    }
}

impl Split for NString {
    type Target = Type;

    // split and replace the current string with the left and right parts
    fn split(&self, offset: u32) -> Result<(Self::Target, Self::Target), String> {
        let data = self.item_ref().borrow().data.clone();
        let (ld, rd) = data.split(offset).unwrap();

        // let split_marks: Vec<(Type, Type)> = self
        //     .item_ref()
        //     .borrow()
        //     .get_marks()
        //     .iter()
        //     .map(|mark| mark.split(offset))
        //     .collect();

        let left_item: Type = ItemRef::new(ld.into(), self.store.clone()).into();
        let right_item: Type = ItemRef::new(rd.into(), self.store.clone()).into();

        // for (l, r) in split_marks {
        //     left_item.item_ref().borrow_mut().add_mark(l);
        //     right_item.item_ref().borrow_mut().add_mark(r);
        // }

        left_item.set_right(right_item.clone());
        right_item.set_left(left_item.clone());
        left_item.set_parent(self.item_ref().borrow().parent.clone());
        right_item.set_parent(self.item_ref().borrow().parent.clone());

        let left = self.item_ref().borrow().left.clone();
        let right = self.item_ref().borrow().right.clone();

        if let Some(left) = left {
            left.set_right(left_item.clone());
            left_item.set_left(left);
        } else if let Some(parent) = self.item_ref().borrow().parent.clone() {
            parent.set_start(left_item.clone());
        }

        if let Some(right) = right {
            right.set_left(right_item.clone());
            right_item.set_right(right);
        }

        self.store
            .upgrade()
            .unwrap()
            .borrow_mut()
            .replace(&self.into(), (left_item.clone(), right_item.clone()));

        Ok((left_item, right_item))
    }
}

impl Serialize for NString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("String", self.borrow().serialize_size() + 1)?;
        self.item.serialize_with(&mut s)?;

        let content = serde_json::to_value(self.content()).unwrap_or_default();
        s.serialize_field("content", &content)?;

        s.end()
    }
}

impl WithId for NString {
    #[inline]
    fn id(&self) -> Id {
        self.item.borrow().id()
    }
}

impl WithIdRange for NString {
    fn range(&self) -> IdRange {
        self.item.borrow().id().range(self.size())
    }
}

impl Deref for NString {
    type Target = ItemRef;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl From<ItemRef> for NString {
    fn from(item: ItemRef) -> Self {
        Self { item }
    }
}

#[cfg(test)]
mod test {
    use crate::doc::Doc;
    use crate::id::{Id, Split};
    use crate::mark::Mark;
    use crate::print_yaml;

    #[test]
    fn test_split_string() {
        let doc = Doc::default();

        let text = doc.text();
        doc.set("text", text.clone());

        let string = doc.string("hello world");
        text.append(string.clone());

        string.add_mark(Mark::Bold);
        string.split(5).unwrap();

        let ls = doc.find_by_id(&Id::new(0, 3)).unwrap();
        // println!("{}", serde_json::to_string(&ls).unwrap());
        let (l, r) = ls.split(2);
        r.add_mark(Mark::Code);
        l.delete();

        // let yaml = serde_yaml::to_string(&doc).unwrap();
        // println!("{}", yaml);
    }

    #[test]
    fn test_merge_string() {
        let doc = Doc::default();
        let text = doc.text();
        doc.set("text", text.clone());

        let string = doc.string("he");
        text.append(string.clone());

        let string = doc.string("llo");
        text.append(string.clone());

        let string = doc.string(" world");
        text.append(string.clone());

        let string = doc.string("!");
        text.append(string.clone());

        print_yaml(&text);
    }
}
