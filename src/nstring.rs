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
use serde::ser::SerializeStruct;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug)]
enum StringContent {
    Small(([u8; 5], u8)),
    Large(String),
}

impl Default for StringContent {
    fn default() -> Self {
        StringContent::Small(([0; 5], 0))
    }
}

impl StringContent {
    fn from(content: String) -> Self {
        if content.len() > 5 {
            StringContent::Large(content)
        } else {
            let mut ins = StringContent::Small(([0; 5], 0));
            ins.insert(0, content);
            ins
        }
    }

    fn insert(&mut self, index: usize, content: String) {
        if self.size() + content.len() >= 5 {}
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
}

#[derive(Debug)]
pub(crate) struct InnerString {
    pub(crate) flags: u8,
    pub(crate) data: AItem,
    content: StringContent,
}

impl InnerString {}

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
        let data = AItem::new(id).with_kind(ItemKind::List);
        Self {
            inner: Rc::new(RefCell::new(InnerString {
                flags: 0,
                data,
                content: StringContent::from(string),
            })),
        }
    }

    #[inline]
    pub(crate) fn content(&self) -> Content {
        self.borrow().content()
    }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        self.borrow().content.size()
    }

    // delete string
    #[inline]
    pub(crate) fn delete(&self) {
        self.item.delete(self.size());
    }

    // pub(crate) fn

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

    pub(crate) fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();

        // map.insert("text".to_string(), self.borrow().content().to_json());

        map.into()
    }
}

impl Split for NString {
    type Target = Type;

    // split and replace the current string with the left and right parts
    fn split(&self, offset: u32) -> Result<(Self::Target, Self::Target), String> {
        let data = self.borrow().data.clone();
        let (ld, rd) = data.split(offset).unwrap();

        // let split_marks: Vec<(Type, Type)> = self
        //     .item_ref()
        //     .borrow()
        //     .get_marks()
        //     .iter()
        //     .map(|mark| mark.split(offset))
        //     .collect();

        let left_item: Type = NString::new(ld.into(), self.store.clone()).into();
        let right_item: Type = NString::new(rd.into(), self.store.clone()).into();

        // for (l, r) in split_marks {
        //     left_item.item_ref().borrow_mut().add_mark(l);
        //     right_item.item_ref().borrow_mut().add_mark(r);
        // }

        // left_item.set_right(right_item.clone());
        // right_item.set_left(left_item.clone());
        // left_item.set_parent(self.item_ref().borrow().parent.clone());
        // right_item.set_parent(self.item_ref().borrow().parent.clone());

        // let left = self.item_ref().borrow().left.clone();
        // let right = self.item_ref().borrow().right.clone();

        // if let Some(left) = left {
        //     left.set_right(left_item.clone());
        //     left_item.set_left(left);
        // } else if let Some(parent) = self.item_ref().borrow().parent.clone() {
        //     parent.set_start(left_item.clone());
        // }

        // if let Some(right) = right {
        //     right.set_left(right_item.clone());
        //     right_item.set_right(right);
        // }

        self.store
            .upgrade()
            .unwrap()
            .borrow_mut()
            .replace(&self.into(), (left_item.clone(), right_item.clone()));

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
