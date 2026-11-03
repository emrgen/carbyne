use carbyne::{Doc, Type};
use std::cmp::Ordering;

#[test]
fn item_ordering_matches_identity_and_sorts() {
    let doc = Doc::default();
    let first: Type = doc.atom("first").into();
    let second: Type = doc.atom("second").into();
    assert_eq!(first.cmp(&second), Ordering::Less);
    assert_eq!(first.partial_cmp(&second), Some(Ordering::Less));
    assert_eq!(second.partial_cmp(&first), Some(Ordering::Greater));
    assert_eq!(first.partial_cmp(&first.clone()), Some(Ordering::Equal));
    assert!(first < second);
    let mut items = vec![second.clone(), first.clone()];
    items.sort();
    assert_eq!(items, vec![first, second]);
}
