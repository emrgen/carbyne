use crate::id::{Id, IdRange, WithId, WithIdRange, WithTarget};
use crate::item::{Content, ItemData, ItemKey, ItemKind};
use crate::natom::AItem;
use crate::nlist::{ListContent, NList};
use crate::nmark::NMark;
use crate::ntext::InnerText;
use crate::store::WeakStoreRef;
use crate::{NString, Type};
use fake::Opt;
use serde::ser::SerializeStruct;
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug)]
pub(crate) struct InnerMove {
    // pub(crate) store: WeakStoreRef,
    pub(crate) flags: u8,
    pub(crate) data: AItem,
    pub(crate) parent: Option<Type>,
    pub(crate) target: Option<Type>,
}

/// NMove represents a move operation in the document.
/// It is similar to proxy but one instance is valid for multiple move operations.
#[derive(Debug, Clone)]
pub(crate) struct NMove {
    inner: Rc<RefCell<InnerMove>>,
}

impl NMove {
    pub(crate) fn new(id: Id, target: Type, store: WeakStoreRef) -> NMove {
        let data = AItem::new(id).with_kind(ItemKind::Text);
        Self {
            inner: Rc::new(RefCell::new(InnerMove {
                // store,
                flags: 0,
                data,
                parent: None,
                target: None,
            })),
        }
    }

    #[inline]
    pub(crate) fn set_parent(&self, parent: Option<Type>) {
        self.borrow_mut().parent = parent;
    }

    #[inline]
    pub(crate) fn get_parent(&self) -> Option<Type> {
        self.borrow_mut().parent.clone()
    }

    #[inline]
    pub(crate) fn depth(&self) -> usize {
        self.borrow().parent.as_ref().map_or(0, |t| t.depth() + 1)
    }

    // #[inline]
    // pub(crate) fn content(&self) -> Content {
    // if let Some(target) = self.get_target().as_ref() {
    //     target.content()
    // } else {
    //     Content::Null
    // }
    // }

    #[inline]
    pub(crate) fn size(&self) -> usize {
        if let Some(target) = self.get_target().as_ref() {
            target.size()
        } else {
            0
        }
    }

    #[inline]
    fn get(&self, key: ItemKey) -> Option<Type> {
        if let Some(target) = self.get_target().as_ref() {
            target.get(key)
        } else {
            None
        }
    }

    #[inline]
    fn set(&self, key: String, item: Type) {
        if let Some(target) = self.get_target().as_ref() {
            target.set(key, item);
        }
    }

    #[inline]
    fn delete(&self) {
        unimplemented!("NMove::delete is not implemented yet")
    }

    #[inline]
    fn prepend(&self, item: Type) {
        if let Some(target) = self.get_target().as_ref() {
            target.prepend(item);
        }
    }

    #[inline]
    fn append(&self, item: Type) {
        if let Some(target) = self.get_target().as_ref() {
            target.append(item);
        }
    }

    #[inline]
    fn insert(&self, offset: usize, item: Type) {
        if let Some(target) = self.get_target().as_ref() {
            target.insert(offset, item);
        }
    }

    #[inline]
    fn remove(&self, key: ItemKey) {
        if let Some(target) = self.get_target().as_ref() {
            target.remove(key);
        }
    }

    #[inline]
    fn clear(&self) {
        if let Some(target) = self.get_target().as_ref() {
            target.clear();
        }
    }

    #[inline]
    pub(crate) fn to_json(&self) -> serde_json::Value {
        if let Some(target) = self.get_target().as_ref() {
            serde_json::Value::Null
            // target.to_json()
        } else {
            serde_json::Value::Null
        }
    }
}

impl Serialize for NMove {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.get_target().serialize(serializer).into()
    }
}

impl WithTarget for NMove {
    #[inline]
    fn set_target(&self, target: Type) {
        self.borrow_mut().target = Some(target);
    }

    #[inline]
    fn get_target(&self) -> Option<Type> {
        self.borrow().target.clone()
    }
}

impl Hash for NMove {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl WithId for NMove {
    #[inline]
    fn id(&self) -> Id {
        self.borrow().data.id()
    }
}

impl WithIdRange for NMove {
    #[inline]
    fn range(&self) -> IdRange {
        self.id().range(1)
    }
}

impl Deref for NMove {
    type Target = Rc<RefCell<InnerMove>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{print_yaml, Doc};

    fn get_text(item: &Type) -> String {
        if let Some(target) = item.get_target() {
            target.text_content()
        } else {
            item.text_content()
        }
    }

    fn get_list_text(list: &NList) -> Vec<String> {
        list.borrow()
            .content
            .iter()
            .filter(|t| t.is_visible())
            .map(get_text)
            .collect()
    }

    macro_rules! append {
        ($list:expr, $($item:expr),+) => {
            $(
                $list.append($item.clone());
            )+
        };

    }

    #[test]
    fn test_move_within_list_to_position() {
        let doc = Doc::default();
        let list = doc.list();
        doc.set("list", list.clone());
        let a = doc.atom("a");
        let b = doc.atom("b");
        let c = doc.atom("c");

        append!(list, a, b, c);

        // print_yaml(&list);

        let at: Type = a.into();
        at.move_to(&list, 3);
        assert_eq!(get_list_text(&list), vec!["b", "c", "a"]);

        at.move_to(&list, 0);
        assert_eq!(get_list_text(&list), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_relative_move_within_list() {
        let doc = Doc::default();
        let list = doc.list();
        doc.set("list", list.clone());

        let a: Type = doc.atom("a").into();
        let b: Type = doc.atom("b").into();
        let c: Type = doc.atom("c").into();
        let d: Type = doc.atom("d").into();

        append!(list, a, b, c, d);
        assert_eq!(get_list_text(&list), vec!["a", "b", "c", "d"]);

        a.move_after(&b);
        assert_eq!(get_list_text(&list), vec!["b", "a", "c", "d"]);

        a.move_after(&d);
        assert_eq!(get_list_text(&list), vec!["b", "c", "d", "a"]);

        a.move_before(&b);
        assert_eq!(get_list_text(&list), vec!["a", "b", "c", "d"]);

        a.move_before(&c);
        assert_eq!(get_list_text(&list), vec!["b", "a", "c", "d"]);
    }

    #[test]
    fn test_move_atom_between_lists() {
        let doc = Doc::default();
        let l1 = doc.list();
        let l2 = doc.list();

        doc.set("l1", l1.clone());
        doc.set("l2", l2.clone());

        let a = doc.atom("a");
        let b = doc.atom("b");
        append!(l1, a, b);

        let c = doc.atom("c");
        let d = doc.atom("d");
        append!(l2, c, d);

        let a1: Type = a.into();
        a1.move_to(&l2, 0);

        assert_eq!(get_list_text(&l1), vec!["b"]);
        assert_eq!(get_list_text(&l2), vec!["a", "c", "d"]);

        let b1: Type = b.into();
        b1.move_to(&l2, 2);

        assert_eq!(get_list_text(&l1), vec![] as Vec<String>);
        assert_eq!(get_list_text(&l2), vec!["a", "c", "b", "d"]);
    }
}
