use serde::ser::SerializeStruct;
use serde::Serialize;

use crate::doc::{CloneDeep, Doc};
use crate::item::Content;
use crate::sync::{sync_docs, SyncDirection};
use crate::types::Type;

#[derive(Clone, Debug)]
pub struct RichText {
    pub doc: Doc,
    pub text: Type,
}

impl RichText {
    pub fn new() -> RichText {
        let doc = Doc::default();
        let text = doc.text();
        doc.set("text", text.clone());

        RichText {
            doc,
            text: text.into(),
        }
    }

    pub fn commit(&self) {
        self.doc.commit();
    }

    pub fn sync(&mut self, other: &RichText) {
        sync_docs(&self.doc, &other.doc, SyncDirection::default());
    }

    pub fn insert(&mut self, index: usize, text: &str) -> Type {
        let text = self.doc.string(text);
        self.text.insert(index as u32, text.clone());

        text.into()
    }

    pub(crate) fn to_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
}

impl PartialEq<Self> for RichText {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for RichText {}

impl Serialize for RichText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let text = self
            .text
            .item_ref()
            .borrow()
            .items()
            .iter()
            .map(|item| {
                if let Content::String(s) = item.content() {
                    s
                } else {
                    "".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("");

        text.serialize(serializer)
    }
}

impl CloneDeep for RichText {
    fn clone_deep(&self) -> Self {
        let doc = self.doc.clone_deep();
        doc.update_client();
        let text = doc.get("text").unwrap().clone();
        RichText { doc, text }
    }
}

#[cfg(test)]
mod tests {
    use rand::seq::SliceRandom;

    use super::*;

    fn create_text_pairs() -> (RichText, RichText) {
        let mut t1 = RichText::new();
        let mut t2 = t1.clone_deep();

        t1.insert(0, "w");
        t2.insert(0, "h");

        // print_yaml(&t1.doc.state());
        // print_yaml(&t2.doc.state());

        t1.sync(&t2);
        assert_eq!(t1, t2);

        // print_yaml(&t1.doc);
        // print_yaml(&t2.doc);

        (t1, t2)
    }

    #[test]
    fn test_rich_text_sync_at_start1() {
        let (mut t1, mut t2) = create_text_pairs();

        // print_yaml(&t1.doc.state());
        // print_yaml(&t2.doc.state());

        // 26 letters
        let mut chars = vec!["a", "b", "c"];
        let mut nums = vec!["1"];

        let mut s1: Option<Type> = None;
        for _ in 0..1 {
            for c in &chars {
                let item = t1.doc.string(c.to_string());
                t1.text.prepend(item);
            }
        }

        s1 = None;
        for _ in 0..1 {
            for c in &nums {
                let item = t2.doc.string(c.to_string());
                t2.text.prepend(item);
            }
        }

        t1.sync(&t2);
        assert_eq!(t1.to_string(), t2.to_string());
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_rich_text_sync_at_end() {
        let (mut t1, mut t2) = create_text_pairs();
        // 26 letters
        let mut chars = vec![
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
            "r", "s", "t",
        ];
        let mut size = 2;

        let mut s1: Option<Type> = None;
        for _ in 0..10 {
            for c in &chars {
                let item = t1.doc.string(c.to_string());
                t1.text.append(item);
            }
        }

        chars.reverse();

        s1 = None;
        for _ in 0..10 {
            for c in &chars {
                let item = t2.doc.string(c.to_string());
                t2.text.append(item);
            }
        }

        t1.sync(&t2);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_rich_text_sync_at_start_end() {
        let (mut t1, mut t2) = create_text_pairs();
        // 26 letters
        let mut chars = vec![
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
            "r", "s", "t",
        ];
        let mut size = 2;

        let mut s1: Option<Type> = None;
        for _ in 0..10 {
            for c in &chars {
                let item = t1.doc.string(c.to_string());
                t1.text.append(item);
            }
        }

        chars.reverse();

        s1 = None;
        for _ in 0..10 {
            for c in &chars {
                let item = t2.doc.string(c.to_string());
                t2.text.prepend(item);
            }
        }

        t1.sync(&t2);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_rich_text_sync_at_end_start() {
        let (mut t1, mut t2) = create_text_pairs();
        // 26 letters
        let mut chars = vec![
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q",
            "r", "s", "t",
        ];
        let mut size = 2;

        let mut s1: Option<Type> = None;
        for _ in 0..50 {
            for c in &chars {
                let item = t1.doc.string(c.to_string());
                t1.text.prepend(item);
            }
        }

        chars.reverse();

        s1 = None;
        for _ in 0..50 {
            for c in &chars {
                let item = t2.doc.string(c.to_string());
                t2.text.append(item);
            }
        }

        t1.sync(&t2);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_text_block_split_by_insert() {
        let (mut t1, mut t2) = create_text_pairs();

        t1.text.insert(0, t1.doc.string("012345678"));
        t1.sync(&t2);
        assert_eq!(t1, t2);

        t2.text.insert(3, t2.doc.string("abc"));
        t1.text.insert(3, t1.doc.string("pqr"));

        t1.sync(&t2);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_append_n_chars() {
        let (mut t1, mut t2) = create_text_pairs();

        for i in 0..100 {
            t1.text.append(t1.doc.string(&i.to_string()));
        }
    }

    #[test]
    fn test_sync_three_docs() {
        let mut t1 = RichText::new();
        let mut t2 = t1.clone_deep();
        let mut t3 = t1.clone_deep();

        t1.insert(0, "a");
        t2.insert(0, "b");
        t3.insert(0, "c");

        t1.sync(&t2);
        t1.sync(&t3);
        t2.sync(&t3);

        assert_eq!(t1, t2);
        assert_eq!(t1, t3);
        assert_eq!(t2, t3);

        // print_yaml(&t1.doc);
        // print_yaml(&t2.doc);
        // print_yaml(&t3.doc);
    }

    #[test]
    fn test_sync_three_docs_with_insert() {
        let mut t1 = RichText::new();
        let mut t2 = t1.clone_deep();
        let mut t3 = t1.clone_deep();

        t1.insert(0, "a");
        t2.insert(0, "b");
        t3.insert(0, "c");

        t1.sync(&t2);
        t1.sync(&t3);
        t2.sync(&t3);

        t1.insert(0, "d");
        t2.insert(0, "e");
        t3.insert(0, "f");

        t1.sync(&t2);
        t1.sync(&t3);
        t2.sync(&t3);

        assert_eq!(t1, t2);
        assert_eq!(t1, t3);
        assert_eq!(t2, t3);

        // print_yaml(&t1.doc);
        // print_yaml(&t2.doc);
        // print_yaml(&t3.doc);
    }

    fn sync_all_docs(docs: &Vec<RichText>) {
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                if i != j {
                    sync_docs(&docs[i].doc, &docs[j].doc, SyncDirection::default());
                }
            }
        }
    }

    #[test]
    fn sync_n_docs() {
        let doc = RichText::new();
        let mut docs = vec![];
        for _ in 0..10 {
            docs.push(doc.clone_deep());
        }

        docs[0].insert(0, "a");
        docs[1].insert(0, "b");
        docs[2].insert(0, "c");
        docs[3].insert(0, "d");
        docs[4].insert(0, "e");
        docs[5].insert(0, "f");
        docs[6].insert(0, "g");
        docs[7].insert(0, "h");
        docs[8].insert(0, "i");
        docs[9].insert(0, "j");

        for doc in &docs {
            doc.commit()
        }

        sync_all_docs(&docs);
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                assert_eq!(docs[i], docs[j]);
            }
        }

        docs[0].insert(0, "k");
        docs[1].insert(0, "l");
        docs[2].insert(0, "m");
        docs[3].insert(0, "n");
        docs[4].insert(0, "o");
        docs[5].insert(0, "p");
        docs[6].insert(0, "q");
        docs[7].insert(0, "r");
        docs[8].insert(0, "s");
        docs[9].insert(0, "t");
        for doc in &docs {
            doc.commit()
        }

        sync_all_docs(&docs);
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                assert_eq!(docs[i], docs[j]);
            }
        }

        // print_yaml(&docs[0].doc);
    }

    #[test]
    fn test_sync_n_docs_with_insert() {
        let doc = RichText::new();
        let mut docs = vec![];
        for _ in 0..10 {
            docs.push(doc.clone_deep());
        }

        let mut chars = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

        insert_random_chars(&mut docs[0], &mut chars, 10);
        insert_random_chars(&mut docs[1], &mut chars, 10);
        insert_random_chars(&mut docs[2], &mut chars, 10);
        insert_random_chars(&mut docs[3], &mut chars, 10);
        insert_random_chars(&mut docs[4], &mut chars, 10);

        sync_all_docs(&docs);

        for i in 0..docs.len() {
            for j in 0..docs.len() {
                assert_eq!(docs[i], docs[j]);
            }
        }
    }

    fn insert_random_chars(doc: &mut RichText, chars: &mut Vec<&str>, size: usize) {
        chars.shuffle(&mut rand::thread_rng());
        for _ in 0..size {
            for c in chars.iter() {
                doc.insert(0, c);
            }
        }
    }

    #[test]
    fn remote_slice() {}
}
