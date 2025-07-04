use crate::bimapid::FieldId;
use crate::delete::DeleteItem;
use crate::id::{ClockTick, Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind};
use crate::mark::Mark;
use crate::nmark::NMark;
use crate::nmove::NMove;
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::{Item, NText};
use serde::ser::SerializeStruct;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::str;
use std::str::Utf8Error;

#[derive(Clone, Debug)]
pub struct StringRef(Rc<RefCell<NString>>);
impl StringRef {
    pub(crate) fn write_content(&self, buf: &mut String) {
        self.0.borrow().write_content(buf);
    }
}

impl Hash for StringRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct NString {
    pub(crate) flags: u8,  // 1
    pub(crate) item: Item, // 16
    parent: Option<Type>,  // 8
    content: String,       // 7
}

impl NString {
    pub(crate) fn new(id: Id, content: String, store: WeakStoreRef) -> Self {
        Self {
            flags: 0,
            item: Item::new(id),
            parent: None,
            content,
        }
    }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.content.len()
    }

    #[inline]
    pub(crate) fn set_parent(&mut self, parent: Type) {
        self.parent = Option::from(parent);
    }

    #[inline]
    pub(crate) fn get_parent(&self) -> Option<Type> {
        self.parent.clone().map(Type::from)
    }

    #[inline]
    pub(crate) fn text_content(&self) -> String {
        self.content.clone()
    }

    #[inline]
    pub(crate) fn write_content(&self, buf: &mut String) {
        buf.extend(self.content.chars());
    }

    // add mark to string
    // pub(crate) fn add_mark(&self, mark: Mark) {
    //     // create mark content, which includes the string id range
    //     // e.g. the range of the string "hello" is 0..5 the mark content will be (0..5, mark)
    //     let content = MarkContent::new(self.id().range(self.size()), mark.clone());
    //     // get next id within the range of the string
    //     let id = self
    //         .store
    //         .upgrade()
    //         .unwrap()
    //         .borrow_mut()
    //         .next_id_range(self.size() as ClockTick)
    //         .id();
    //
    //     let mark = NMark::new(id, Content::Mark(content), self.store.clone());
    //
    //     // self.item_ref().add_mark(mark);
    // }

    pub(crate) fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();

        // map.insert("text".to_string(), self.borrow().content().to_json());

        map.into()
    }
}

// impl Split for NString {
//     type Target = Type;
//
//     // split and replace the current string with the left and right parts
//     fn split(
//         &self,
//         store: &WeakStoreRef,
//         offset: usize,
//     ) -> Result<(Self::Target, Self::Target), String> {
//         // let data = self.borrow().content.content();
//         // let content = self.borrow().content.as_string();
//
//         // let (lid, rid) = self.range().split(offset)?;
//         // let (ld, rd) = content.split_at(offset);
//
//         // let split_marks: Vec<(Type, Type)> = self
//         //     .item_ref()
//         //     .borrow()
//         //     .get_marks()
//         //     .iter()
//         //     .map(|mark| mark.split(offset))
//         //     .collect();
//
//         let left_item: Type = NString::new(lid.id(), ld.into(), store.clone()).into();
//         let right_item: Type = NString::new(rid.id(), rd.into(), store.clone()).into();
//
//         // let self_type = self.into();
//         //
//         // if let Some(parent) = &self.borrow().parent {
//         //     parent.replace(&self_type, (left_item.clone(), right_item.clone()));
//         // }
//         //
//         // store
//         //     .upgrade()
//         //     .unwrap()
//         //     .borrow_mut()
//         //     .replace(&self_type, (left_item.clone(), right_item.clone()));
//
//         Ok((left_item, right_item))
//     }
// }

impl Eq for NString {}

impl PartialEq for NString {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.size() == other.size()
    }
}

impl Serialize for NString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("String", 1)?;
        // self.item.serialize_with(&mut s)?;

        // let content = serde_json::to_value(self.content()).unwrap_or_default();
        // s.serialize_field("content", &content)?;

        s.end()
    }
}

impl Hash for NString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl WithId for NString {
    #[inline]
    fn id(&self) -> Id {
        self.id
    }
}

impl WithIdRange for NString {
    fn range(&self) -> IdRange {
        self.id.range(self.size() as u32)
    }
}

impl Deref for NString {
    type Target = Item;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}
impl DerefMut for NString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
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

        // string.add_mark(Mark::Bold);
        // string.split(5).unwrap();

        let ls = doc.find_by_id(&Id::new(0, 3)).unwrap();
        // println!("{}", serde_json::to_string(&ls).unwrap());
        let (l, r) = ls.split(2);
        // r.add_mark(Mark::Code);
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
