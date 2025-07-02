use serde::ser::SerializeStruct;
use serde::Serialize;
use std::cell::RefCell;
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::io::BufRead;
use std::ops::{Deref, Range};
use std::rc::Rc;

use crate::id::{Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind};
use crate::mark::MarkContent;
use crate::natom::AItem;
use crate::nlist::{InnerList, ListContent, NList};
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::NString;

#[derive(Debug)]
pub(crate) struct InnerText {
    pub(crate) store: WeakStoreRef,
    pub(crate) flags: u8,
    pub(crate) data: AItem,
    pub(crate) parent: Option<Type>,
    content: ListContent<NString>,
}

#[derive(Clone, Debug)]
pub struct NText {
    inner: Rc<RefCell<InnerText>>,
}

impl NText {
    pub(crate) fn on_insert(&self, p0: &Type) {}
}

// impl NText {
//     pub(crate) fn slice(&self, start: u32, end: u32) -> Vec<Type> {
//         let start = self.find_at_offset(start);
//         let end = self.find_at_offset(end);
//
//         let store = self.store.upgrade().unwrap();
//         if let Some(item) = &end.0 {
//             let items = item.split(end.1);
//             item.replace(items);
//         }
//
//         if let Some(item) = &start.0 {
//             let items = item.split(start.1);
//             item.replace(items);
//         }
//
//         Vec::new()
//     }
// }

impl NText {
    pub(crate) fn new(id: Id, store: WeakStoreRef) -> Self {
        let data = AItem::new(id).with_kind(ItemKind::Text);
        Self {
            inner: Rc::new(RefCell::new(InnerText {
                store,
                flags: 0,
                data,
                parent: None,
                content: ListContent::default(),
            })),
        }
    }

    pub(crate) fn clear(&self) {
        self.borrow()
            .content
            .iter()
            .filter(|t| t.is_visible())
            .for_each(|item| item.delete());
    }

    // pub(crate) fn content(&self) -> Content {
    //     let items = self.borrow().as_list();
    //     Content::Types(items)
    // }

    pub(crate) fn add_mark(&self, mark: MarkContent, range: Range<usize>) {}

    pub(crate) fn remove_mark(&self, mark: MarkContent, range: Range<usize>) {}

    pub(crate) fn replace(&self, child: &Type, with: (Type, Type)) {}

    // calculate the visible text length
    pub(crate) fn size(&self) -> usize {
        self.borrow()
            .content
            .iter()
            .map(|t| if (t.is_visible()) { t.size() } else { 0 })
            .fold(0, |acc, size| acc + size)
    }

    pub fn append(&self, item: impl Into<Type>) {
        let item = item.into();
        assert!(item.kind().is_string());
        let item = item.as_string().unwrap();
        let content = &mut self.borrow_mut().content;

        // set the left id
        content.last().map(|f| item.set_left_id(f.range().end_id()));

        content.append(item);
    }

    pub fn prepend(&self, item: impl Into<Type>) {
        let item = item.into();
        assert!(item.kind().is_string());
        let content = &mut self.borrow_mut().content;

        // set the right id
        content.first().map(|f| item.set_right_id(f.id()));
        content.prepend(item.as_string().unwrap());
    }

    /// Insert string in text
    pub fn insert(&self, offset: usize, item: impl Into<Type>) {
        let item = item.into();

        assert!(item.kind().is_string());

        if offset == 0 {
            self.prepend(item);
        } else if offset >= self.size() {
            self.append(item);
        } else {
            // find the target item offset
            // let (target, offset) = self.find_at_offset(offset);
            //
            // if let Some(target) = target {
            //     if offset == 0 {
            //         target.insert_before(item);
            //     } else if offset >= target.size() {
            //         target.insert_after(item);
            //     } else {
            //         let items = target.split(offset);
            //         items.0.insert_after(item);
            //     }
            // }
        }
    }

    pub(crate) fn to_json(&self) -> serde_json::Value {
        let items: Vec<_> = self
            .borrow()
            .content
            .iter()
            .filter(|t| t.is_visible())
            .map(|item| item.to_json())
            .collect();

        items.into()
    }

    // raw un marked text content
    pub(crate) fn text_content(&self) -> String {
        let mut ret = String::new();
        self.borrow()
            .content
            .iter()
            .filter(|t| t.is_visible())
            .for_each(|item| item.write_content(&mut ret));

        ret
    }
}

impl Serialize for NText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("Text", 1)?;
        // self.borrow().serialize_with(&mut s)?;

        // let content = self
        //     .visible_item_iter()
        //     .map(|item| serde_json::to_value(item).unwrap_or_default())
        //     .collect::<Vec<_>>();

        // s.serialize_field("content", &serde_json::Value::Array(content))?;

        s.end()
    }
}

impl Deref for NText {
    type Target = Rc<RefCell<InnerText>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl WithIdRange for NText {
    fn range(&self) -> IdRange {
        self.id().range(1)
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

        // assert_eq!(text.item_ref().borrow().items().len(), 4);

        // let text_ref = &text.item_ref();
        // let text_ref = text_ref.borrow();
        // let items = text_ref.items().clone();
        // let first = items.first().unwrap();
        // assert_eq!(
        //     items.first().unwrap().content().to_json(),
        //     "wor".to_string()
        // );
        // assert_eq!(items.get(1).unwrap().content().to_json(), "foo".to_string());
        // assert_eq!(items.get(2).unwrap().content().to_json(), "ld".to_string());
        // assert_eq!(
        //     items.get(3).unwrap().content().to_json(),
        //     "hello".to_string()
        // );
    }
}
