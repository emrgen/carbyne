use std::ops::Deref;

use serde::Serialize;
use serde_json::Value;

use crate::id::{Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind, ItemRef};
use crate::store::WeakStoreRef;
use crate::types::Type;

#[derive(Clone, Debug)]
pub(crate) struct NMark {
    item: ItemRef,
}

impl NMark {
    pub(crate) fn new(id: Id, content: Content, store: WeakStoreRef) -> Self {
        let data = ItemData {
            kind: ItemKind::Mark,
            id,
            content,
            ..ItemData::default()
        };

        let item = ItemRef::new(data.into(), store);

        Self { item }
    }

    pub(crate) fn from_data(data: ItemData, store: WeakStoreRef) -> Self {
        let item = ItemRef::new(data.into(), store);

        Self { item }
    }

    pub(crate) fn size(&self) -> u32 {
        // let marks = self.borrow().get_marks();

        1 //marks.len() as u32
    }

    pub(crate) fn item_ref(&self) -> ItemRef {
        self.item.clone()
    }

    pub(crate) fn to_json(&self) -> Value {
        serde_json::to_value(self.clone()).unwrap()
    }
}

impl NMark {
    pub(crate) fn content(&self) -> Content {
        self.item_ref().borrow().content.clone()
    }
}

impl Split for NMark {
    type Target = Type;

    // split the mark at the given offset and return the two new marks
    fn split(&self, offset: u32) -> Result<(Self::Target, Self::Target), String> {
        let (ld, rd) = self.item_ref().borrow().data.split(offset).unwrap();
        let left = NMark::from_data(ld, self.store.clone());
        let right = NMark::from_data(rd, self.store.clone());

        Ok((left.into(), right.into()))
    }
}

impl Serialize for NMark {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let content = self.content();
        if let Content::Mark(mark) = &content {
            serializer.serialize_some(mark)
        } else {
            panic!("NMark content is not a mark type");
        }
    }
}

impl Deref for NMark {
    type Target = ItemRef;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl WithId for NMark {
    #[inline]
    fn id(&self) -> Id {
        self.item.borrow().id()
    }
}

impl WithIdRange for NMark {
    fn range(&self) -> IdRange {
        self.borrow().id().range(1)
    }
}

impl From<ItemRef> for NMark {
    fn from(item: ItemRef) -> Self {
        Self { item }
    }
}

#[cfg(test)]
mod tests {
    use crate::doc::Doc;
    use crate::mark::Mark;

    #[test]
    fn test_nmark() {
        let doc = Doc::default();

        doc.add_mark(Mark::Bold);
        doc.add_mark(Mark::Italic);
        doc.add_mark(Mark::Color("red".into()));

        let yaml = serde_yaml::to_string(&doc).unwrap();
        println!("{}", yaml);
        // let marks = doc.root.borrow().get_marks();

        // let yaml = serde_yaml::to_string(&marks).unwrap();
        // println!("{}", yaml);
    }

    #[test]
    fn test_mark_string() {
        let doc = Doc::default();
        let s1 = doc.string("hello");
        s1.add_mark(Mark::Bold);

        let yaml = serde_yaml::to_string(&s1).unwrap();
        println!("{}", yaml);
    }
}
