use crate::doc::Doc;
use crate::print_yaml;

pub fn equal_docs(d1: &Doc, d2: &Doc) -> bool {
    let left = serde_json::to_string(d1).unwrap();
    let right = serde_json::to_string(d2).unwrap();

    // println!("left: {}", left);
    // println!("right: {}", right);

    left == right
}

#[derive(Debug, PartialEq, Default)]
pub enum SyncDirection {
    LeftToRight,
    RightToLeft,
    #[default]
    Both,
}

pub fn sync_docs(d1: &Doc, d2: &Doc, direction: SyncDirection) {
    let diff1 = d1.diff(d2);
    let diff2 = d2.diff(d1);

    // println!("diff1");
    // print_yaml(&diff1);
    // println!("diff2");
    // print_yaml(&diff2);

    if direction == SyncDirection::LeftToRight {
        println!("sync_docs: d1 -> d2");
        d2.apply(&diff1);
    } else if direction == SyncDirection::RightToLeft {
        d1.apply(&diff2);
    } else {
        println!("sync_docs: d1 -> d2");
        d1.apply(&diff2);
        println!("sync_docs: d2 -> d1");
        d2.apply(&diff1);
        println!("sync_docs: done");
    }
}

pub fn sync_first_doc(d1: &Doc, d2: &Doc) {
    let diff1 = d2.diff(d1);
    d1.apply(&diff1);
}

#[cfg(test)]
mod test {
    use crate::doc::{CloneDeep, Doc};
    use crate::print_yaml;
    use crate::sync::{equal_docs, sync_docs, SyncDirection};
    use rand::prelude::SliceRandom;
    use rand::Rng;
    use serde_json::json;

    #[test]
    fn test_sync_lr() {
        let d1 = Doc::default();
        let d2 = d1.clone_deep();
        d2.update_client();

        d1.set("a", d1.string("hello"));
        d1.commit();
        // d2.set("b", d2.string("world"));
        // d2.commit();

        sync_docs(&d1, &d2, SyncDirection::LeftToRight);
        assert!(equal_docs(&d1, &d2));
    }

    #[test]
    fn test_sync_rl() {
        let d1 = Doc::default();
        let d2 = d1.clone_deep();
        d2.update_client();

        // d1.set("a", d1.string("hello"));
        // d1.commit();
        d2.set("b", d2.string("world"));
        d2.commit();

        sync_docs(&d1, &d2, SyncDirection::RightToLeft);
        assert!(equal_docs(&d1, &d2));
    }

    #[test]
    fn test_sync11() {
        let d1 = Doc::default();
        let d2 = d1.clone_deep();
        d2.update_client();

        let list = d1.list();
        d1.set("list", list.clone());

        list.append(d1.atom("a"));
        d1.commit();
        list.append(d1.atom("b"));
        d1.commit();

        sync_docs(&d1, &d2, SyncDirection::default());
        assert!(equal_docs(&d1, &d2));
    }

    #[test]
    fn test_sync2() {
        let doc1 = Doc::default();
        let doc2 = doc1.clone_deep();
        doc2.update_client();

        doc1.set("a", doc1.string("hello"));
        doc1.commit();

        doc2.set("a", doc2.string("world"));
        doc2.commit();

        sync_docs(&doc1, &doc2, SyncDirection::default());

        assert!(equal_docs(&doc1, &doc2));
    }

    #[test]
    fn test_doc_commit_rollback_map_field() {
        let doc = Doc::default();

        doc.set("a", doc.string("hello"));
        doc.commit();

        assert_eq!(doc.get("a").unwrap().to_json(), json!({"text": "hello"}));

        doc.set("b", doc.string("world"));
        assert_eq!(doc.get("b").unwrap().to_json(), json!({"text": "world"}));

        // should rollback b field
        doc.rollback();
        assert_eq!(doc.get("b").is_some(), false)
    }

    #[test]
    fn test_doc_commit_rollback_list_element() {
        let doc = Doc::default();
        let list = doc.list();
        doc.set("list", list.clone());
        doc.commit();

        list.append(doc.string("hello"));

        assert_eq!(list.size(), 1);
        doc.rollback();
        assert_eq!(list.size(), 0);

        for i in 0..5 {
            list.append(doc.string(i.to_string()));
        }
        doc.commit();

        for i in 5..15 {
            list.append(doc.string(i.to_string()));
        }

        assert_eq!(list.size(), 15);
        doc.rollback();
        assert_eq!(list.size(), 5);
    }

    #[test]
    fn test_sync_with_list() {
        let doc1 = Doc::default();
        let doc2 = doc1.clone_deep();
        doc2.update_client();

        let list1 = doc1.list();
        doc1.set("list1", list1.clone());

        list1.append(doc1.string("a"));
        list1.append(doc1.string("b"));
        doc1.commit();

        let list2 = doc2.list();
        doc2.set("list2", list2.clone());

        list2.append(doc2.string("a"));

        sync_docs(&doc1, &doc2, SyncDirection::default());
        assert!(equal_docs(&doc1, &doc2));

        let list1 = doc2.get("list1").unwrap().as_list().unwrap();
        list1.append(doc2.string("c"));
        doc2.commit();

        sync_docs(&doc1, &doc2, SyncDirection::default());

        // print_yaml(&doc1);
        // print_yaml(&doc2);

        assert!(equal_docs(&doc1, &doc2));
    }

    #[test]
    fn test_sync_with_text() {
        let doc1 = Doc::default();
        let doc2 = doc1.clone_deep();
        doc2.update_client();

        let text1 = doc1.text();
        doc1.set("text", text1.clone());

        sync_docs(&doc1, &doc2, SyncDirection::default());
        assert!(equal_docs(&doc1, &doc2));

        text1.insert(0, doc1.string("a"));

        sync_docs(&doc1, &doc2, SyncDirection::default());
        assert!(equal_docs(&doc1, &doc2));

        let text2 = doc2.get("text").unwrap().as_text().unwrap();
        let text1 = doc1.get("text").unwrap().as_text().unwrap();

        text2.insert(1, doc2.string("b"));
        text2.insert(0, doc2.string("c"));

        text1.insert(1, doc1.string("d"));

        sync_docs(&doc1, &doc2, SyncDirection::default());
        // print_yaml(&doc1);
        // print_yaml(&doc2);
        assert!(equal_docs(&doc1, &doc2));
    }

    #[test]
    fn test_sync_with_text2() {
        let doc1 = Doc::default();
        let doc2 = doc1.clone_deep();
        doc2.update_client();

        let text = doc1.text();
        doc1.set("text", text.clone());
        sync_docs(&doc1, &doc2, SyncDirection::default());

        let text2 = doc2.get("text").unwrap().as_text().unwrap();
        let text1 = doc1.get("text").unwrap().as_text().unwrap();

        // character vector 26
        let chars = vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ];
        // shuffle the characters
        let mut chars1 = chars.clone();
        let mut rng = rand::thread_rng();
        chars1.shuffle(&mut rng);

        let mut chars2 = chars.clone();
        let mut rng = rand::thread_rng();
        chars2.shuffle(&mut rng);

        for (a, b) in chars1.iter().zip(chars2.iter()) {
            let size1 = text1.size();
            let size2 = text2.size();
            // randomly insert the characters
            let pos1 = rng.gen_range(0..size1 + 1);
            let pos2 = rng.gen_range(0..size2 + 1);
            text1.insert(pos1, doc1.string(&a.to_string()));
            text2.insert(pos2, doc2.string(&b.to_string()));

            // random bool
            let sync = rng.gen_bool(0.5);
            if sync {
                sync_docs(&doc1, &doc2, SyncDirection::default());
            }
        }

        sync_docs(&doc1, &doc2, SyncDirection::default());

        // print_yaml(&text1);
        // print_yaml(&text2);

        // assert!(equal_docs(&doc1, &doc2));
    }

    // #[test]
    // fn test_inser
}
