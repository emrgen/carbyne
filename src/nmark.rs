use serde::{Serialize, Serializer};
use serde_json::Value;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind};
use crate::nmap::NMap;
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::{Item, NString};

#[derive(Clone, Debug)]
pub(crate) struct MarkRef {
    inner: Rc<RefCell<NMark>>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub(crate) enum Mark {
    Bold,
    Italic,
    Underline,
    StrikeThrough,
    Code,
    Subscript,
    Superscript,
    Color(Id),
    Background(Id),
    Link(Id),
    Custom(Id),
    #[default]
    None,
}

impl Serialize for Mark {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Mark::Bold => "bold".serialize(serializer),
            Mark::Italic => "italic".serialize(serializer),
            Mark::Underline => "underline".serialize(serializer),
            Mark::StrikeThrough => "strikethrough".serialize(serializer),
            Mark::Code => "code".serialize(serializer),
            Mark::Subscript => "subscript".serialize(serializer),
            Mark::Superscript => "superscript".serialize(serializer),
            Mark::Color(ref color) => color.serialize(serializer),
            Mark::Background(ref color) => color.serialize(serializer),
            Mark::Link(ref url) => url.serialize(serializer),
            Mark::Custom(ref custom) => custom.serialize(serializer),
            Mark::None => "_".serialize(serializer),
        }
    }
}

impl Encode for Mark {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {}
}

impl Decode for Mark {
    fn decode<D: Decoder>(d: &mut D, ctx: &DecodeContext) -> Result<Mark, String> {
        todo!()
    }
}

#[derive(Debug)] // 24
pub(crate) struct NMark {
    pub(crate) flags: u8,  // 1
    pub(crate) item: Item, // 16
    content: Mark,         // 5
}

impl NMark {
    pub(crate) fn new(id: Id, content: Mark, store: WeakStoreRef) -> Self {
        Self {
            flags: 0,
            item: Item::new(id),
            content,
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
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

impl DerefMut for NMark {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
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
        self.id
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
    use crate::nmark::Mark;

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
