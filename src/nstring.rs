use crate::bimapid::FieldId;
use crate::delete::DeleteItem;
use crate::id::{ClockTick, Id, IdRange, Split, WithId, WithIdRange};
use crate::item::{Content, ItemData, ItemKind, ItemRef};
use crate::mark::{Mark, MarkContent};
use crate::natom::AItem;
use crate::nlist::{InnerList, ListContent};
use crate::nmark::NMark;
use crate::nmove::NMove;
use crate::store::WeakStoreRef;
use crate::types::Type;
use crate::NText;
use serde::ser::SerializeStruct;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::ops::Deref;
use std::rc::Rc;
use std::str;
use std::str::Utf8Error;

#[derive(Debug)]
enum StringContent {
    Small(([u8; 6], u8)),
    Large(String),
}

impl Default for StringContent {
    fn default() -> Self {
        StringContent::Small(([0; 6], 0))
    }
}

impl StringContent {
    fn from(content: String) -> Self {
        if content.len() > 6 {
            StringContent::Large(content)
        } else {
            let mut ins = StringContent::Small(([0; 6], 0));
            ins.insert(0, content);
            ins
        }
    }

    fn insert(&mut self, index: usize, content: String) {
        // if self.size() + content.len() >= 6 {}
        match self {
            StringContent::Small(sm) => {}
            StringContent::Large(lg) => {
                lg.insert_str(index, &content);
            }
        }
    }

    fn size(&self) -> usize {
        match self {
            StringContent::Small(sm) => sm.1 as usize,
            StringContent::Large(lg) => lg.len(),
        }
    }

    fn content(&self) -> &[u8] {
        match self {
            StringContent::Small(sm) => &sm.0,
            StringContent::Large(lg) => lg.as_bytes(),
        }
    }

    fn as_string(&self) -> String {
        match str::from_utf8(self.content()) {
            Ok(s) => s.to_string(),
            Err(_) => panic!("NString can't be empty"),
        }
    }

    fn write_content(&self, buf: &mut String) {
        match str::from_utf8(self.content()) {
            Ok(s) => buf.push_str(s),
            Err(_) => panic!("NString can't be empty"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct InnerString {
    pub(crate) flags: u8,   // 1
    pub(crate) data: AItem, // 16
    parent: Option<NText>,  // 8
    content: StringContent, // 7
}

impl InnerString {
    fn delete(&mut self) {}

    fn is_visible(&self) -> bool {
        self.flags | 0x01 != 0
    }
}

impl WithId for InnerString {
    fn id(&self) -> Id {
        self.data.id()
    }
}

#[derive(Clone, Debug)]
pub struct NString {
    inner: Rc<RefCell<InnerString>>,
}

impl NString {
    pub(crate) fn new(id: Id, string: String, store: WeakStoreRef) -> Self {
        let data = AItem::new(id);
        Self {
            inner: Rc::new(RefCell::new(InnerString {
                flags: 0,
                data,
                parent: None,
                content: StringContent::from(string),
            })),
        }
    }

    #[inline]
    pub(crate) fn content(&self) -> Content {
        Content::String(self.borrow().content.as_string())
    }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.borrow().content.size()
    }

    #[inline]
    pub(crate) fn is_visible(&self) -> bool {
        self.borrow().is_visible()
    }

    #[inline]
    pub(crate) fn set_field_id(&self, field_id: FieldId) {
        self.borrow_mut().data.set_field(field_id);
    }

    #[inline]
    pub(crate) fn get_field_id(&self) -> Option<FieldId> {
        self.borrow_mut().data.field_id().clone()
    }

    #[inline]
    pub(crate) fn set_left_id(&self, left_id: Id) {
        self.borrow_mut().data.set_left(left_id);
    }

    #[inline]
    pub(crate) fn get_left_id(&self) -> Option<Id> {
        self.borrow_mut().data.left_id().clone()
    }

    #[inline]
    pub(crate) fn set_right_id(&self, right_id: Id) {
        self.borrow_mut().data.set_right(right_id);
    }

    #[inline]
    pub(crate) fn get_right_id(&self) -> Option<Id> {
        self.borrow_mut().data.right_id().clone()
    }

    #[inline]
    pub(crate) fn set_parent_id(&self, parent_id: Id) {
        self.borrow_mut().data.set_parent(parent_id);
    }

    #[inline]
    pub(crate) fn get_parent_id(&self) -> Option<Id> {
        self.borrow_mut().data.parent_id().clone()
    }

    #[inline]
    pub(crate) fn delete(&self) {
        self.borrow_mut().delete();
    }

    pub(crate) fn text_content(&self) -> String {
        self.borrow().content.as_string()
    }

    pub(crate) fn write_content(&self, buf: &mut String) {
        self.borrow().content.write_content(buf);
    }

    // pub(crate) fn

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

impl Split for NString {
    type Target = Type;

    // split and replace the current string with the left and right parts
    fn split(
        &self,
        store: &WeakStoreRef,
        offset: usize,
    ) -> Result<(Self::Target, Self::Target), String> {
        let data = self.borrow().content.content();
        let content = self.borrow().content.as_string();

        let (lid, rid) = self.range().split(offset)?;
        let (ld, rd) = content.split_at(offset);

        // let split_marks: Vec<(Type, Type)> = self
        //     .item_ref()
        //     .borrow()
        //     .get_marks()
        //     .iter()
        //     .map(|mark| mark.split(offset))
        //     .collect();

        let left_item: Type = NString::new(lid.id(), ld.into(), store.clone()).into();
        let right_item: Type = NString::new(rid.id(), rd.into(), store.clone()).into();

        // let self_type = self.into();
        //
        // if let Some(parent) = &self.borrow().parent {
        //     parent.replace(&self_type, (left_item.clone(), right_item.clone()));
        // }
        //
        // store
        //     .upgrade()
        //     .unwrap()
        //     .borrow_mut()
        //     .replace(&self_type, (left_item.clone(), right_item.clone()));

        Ok((left_item, right_item))
    }
}

impl Eq for NString {}

impl PartialEq for NString {
    fn eq(&self, other: &Self) -> bool {
        self.borrow().id() == other.borrow().id() && self.size() == other.size()
    }
}

impl Serialize for NString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut s = serializer.serialize_struct("String", 1)?;
        // self.item.serialize_with(&mut s)?;

        let content = serde_json::to_value(self.content()).unwrap_or_default();
        s.serialize_field("content", &content)?;

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
        self.borrow().id()
    }
}

impl WithIdRange for NString {
    fn range(&self) -> IdRange {
        self.borrow().id().range(self.size() as u32)
    }
}

impl Deref for NString {
    type Target = Rc<RefCell<InnerString>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
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
