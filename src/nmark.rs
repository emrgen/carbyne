use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

use crate::id::{Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind, ItemRef};
use crate::natom::AItem;
use crate::nlist::ListContent;
use crate::nmap::NMap;
use crate::ntext::InnerText;
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::NString;

#[derive(Debug)]
enum MarkContent {
    Bold,
    Italic,
    Underline,
    Strike,
    Subscript,
    Superscript,
    Color(u16),
    Background(u16),
    Custom(u16),
}

#[derive(Debug)] // 24
pub(crate) struct InnerMark {
    pub(crate) flags: u8,   // 1
    pub(crate) data: AItem, // 16
    content: MarkContent,   // 5
}

#[derive(Clone, Debug)]
pub(crate) struct NMark {
    inner: Rc<RefCell<InnerMark>>,
}

impl NMark {
    pub(crate) fn new(id: Id, content: MarkContent, store: WeakStoreRef) -> Self {
        let data = AItem::new(id).with_kind(ItemKind::Text);
        Self {
            inner: Rc::new(RefCell::new(InnerMark {
                flags: 0,
                data,
                content,
            })),
        }
    }

    // pub(crate) fn from_data(data: ItemData, store: WeakStoreRef) -> Self {
    //     let item = ItemRef::new(data.into(), store);
    //
    //     Self { item }
    // }

    pub(crate) fn size(&self) -> usize {
        1 //marks.len() as u32
    }

    pub(crate) fn to_json(&self) -> Value {
        serde_json::to_value(self.clone()).unwrap()
    }

    // pub(crate) fn content(&self) -> Content {
    //     self.item_ref().borrow().content.clone()
    // }

    fn split(&self, offset: u32) -> Result<(Type, Type), String> {
        // let (ld, rd) = self.item_ref().borrow().data.split(offset).unwrap();
        // let left = NMark::from_data(ld, self.store.clone());
        // let right = NMark::from_data(rd, self.store.clone());

        // Ok((left.into(), right.into()))
        unimplemented!()
    }
}

impl Serialize for NMark {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        unimplemented!()
        // let content = self.content();
        // if let Content::Mark(mark) = &content {
        //     serializer.serialize_some(mark)
        // } else {
        //     panic!("NMark content is not a mark type");
        // }
    }
}

impl Deref for NMark {
    type Target = Rc<RefCell<InnerMark>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Hash for NMark {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl WithId for NMark {
    #[inline]
    fn id(&self) -> Id {
        self.borrow().data.id()
    }
}

impl WithIdRange for NMark {
    fn range(&self) -> IdRange {
        self.id().range(1)
    }
}

#[cfg(test)]
mod tests {
    use crate::doc::Doc;
    use crate::mark::Mark;

    #[test]
    fn test_mark() {
        let doc = Doc::default();

        doc.add_mark(Mark::Bold);
        doc.add_mark(Mark::Italic);
        // doc.add_mark(Mark::Color("red".into()));

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
        // s1.add_mark(Mark::Bold);

        let yaml = serde_yaml::to_string(&s1).unwrap();
        println!("{}", yaml);
    }
}
