use carbyne::Doc;

#[test]
fn typed_mutations_preserve_shared_container_state() {
    let doc = Doc::default();
    let map = doc.map();
    doc.set("map", map.clone());
    let dynamic = doc.get("map").unwrap();
    assert!(dynamic.as_list().is_none());
    let map = dynamic.as_map().unwrap();
    map.set("title", doc.atom("value"));
    assert_eq!(map.size(), 1);
    assert!(map.get("title").is_some());
    map.remove("title");
    assert!(map.get("title").is_none());
    let list = doc.list();
    doc.set("list", list.clone());
    let dynamic = doc.get("list").unwrap();
    dynamic.as_list().unwrap().append(doc.atom("one"));
    assert_eq!(list.size(), 1);
    assert!(list.get(0u32).is_some());
}
