use carbyne::{CloneDeep, Doc, Type};

fn main() {
    let doc1 = Doc::default();
    let mut l1: Type = doc1.list().into();
    doc1.set("list1", l1.clone());

    let doc2 = doc1.clone_deep();
    let mut l2 = doc2.get("list1").unwrap().clone();

    assert_eq!(doc1, doc2);
    doc2.update_client();

    // 26 letters
    let mut chars = vec![
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", //"u", "v", "w", "x", "y", "z",
    ];
    let mut size = 2;
    let now = std::time::Instant::now();

    let mut size = 1;
    for _ in 0..5000 {
        for c in &chars {
            let item = doc1.string(c.to_string());
            // random index
            let index = rand::random::<usize>() % size;

            l1.as_list().unwrap().insert(index as u32, item);
            // l1.append(item);
            size += 1;
        }
    }

    println!("first doc: {:?}", now.elapsed());

    println!("{:?}", l1.size());
    chars.reverse();

    let now = std::time::Instant::now();

    let mut size = 1;
    for _ in 0..5000 {
        for c in &chars {
            let item = doc2.string(c.to_string());
            // random index
            let index = rand::random::<usize>() % size;
            l2.as_list().unwrap().append(item);
            size += 1;
        }
    }

    println!("second doc: {:?}", now.elapsed());

    let now = std::time::Instant::now();

    // sync_docs(&doc1, &doc2, SyncDirection::LeftToRight);
    // sync_docs(&t1.doc, &t2.doc, SyncDirection::RightToLeft);

    // sync_docs(&doc1, &doc2, SyncDirection::Both);

    println!("{:?}", now.elapsed());

    println!("{:?}", l1.size());
    println!("{:?}", l2.size());

    // if let Content::Types(list) = l1.content() {
    //     let content = list
    //         .iter()
    //         .map(|item| match item.content() {
    //             Content::String(s) => s,
    //             _ => "".parse().unwrap(),
    //         })
    //         .collect::<Vec<_>>()
    //         .join("");
    //
    //     println!("{:?}", content);
    // }
    //
    // if let Content::Types(list) = l2.content() {
    //     let content = list
    //         .iter()
    //         .map(|item| match item.content() {
    //             Content::String(s) => s,
    //             _ => "".parse().unwrap(),
    //         })
    //         .collect::<Vec<_>>()
    //         .join("");
    //
    //     println!("{:?}", content);
    // }

    // for i in 0..l2.size() {
    //     let item1 = l1.get(i).unwrap();
    //     let item2 = l2.get(i).unwrap();
    //     println!("{:?}: {:?}", item1.content(), item2.content());
    //     // assert_eq!(item1, item2);
    // }

    // assert_eq!(doc1, doc2);

    // print_yaml(l1.content());
    // print_yaml(l2.content());
}
