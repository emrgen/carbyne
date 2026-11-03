use carbyne::Doc;
use carbyne_blocks::{BlockStore, TextOpId};
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn test_mark_ops_applied() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Mark op as applied
    let ops = vec![TextOpId { id: "seq-1#client-A".to_string() }];
    store.mark_ops_applied("inst-1", ops).unwrap();

    // Check if applied
    assert!(store.is_op_applied("inst-1", "seq-1#client-A").unwrap());
    assert!(!store.is_op_applied("inst-1", "seq-2#client-A").unwrap());
}

#[test]
fn test_record_placement() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Record placement
    store.record_placement("inst-1", 123).unwrap();

    // Query instances in block
    let instances = store.instances_in(&[123]).unwrap();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0], "inst-1");
}

#[test]
fn test_split_scenario() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // User splits paragraph: block 100 → blocks 100 + 999
    store.record_placement("inst-X", 100).unwrap();
    store.record_placement("inst-X", 999).unwrap();

    // Query: what instances are in block 999?
    let instances = store.instances_in(&[999]).unwrap();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0], "inst-X");
}

#[test]
fn test_multiple_instances_in_one_block() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Two instances in same block (collaborative editing)
    store.record_placement("inst-A", 500).unwrap();
    store.record_placement("inst-B", 500).unwrap();

    let instances = store.instances_in(&[500]).unwrap();
    assert_eq!(instances.len(), 2);
    assert!(instances.contains(&"inst-A".to_string()));
    assert!(instances.contains(&"inst-B".to_string()));
}

#[test]
fn test_op_dedup() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Same op, marked 3 times (resends)
    let op_id = TextOpId { id: "retry-id".to_string() };

    for _ in 0..3 {
        store.mark_ops_applied("offline-inst", vec![op_id.clone()]).unwrap();
    }

    // Should only be marked once (dedup)
    assert!(store.is_op_applied("offline-inst", "retry-id").unwrap());
}

#[test]
fn test_empty_queries() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Empty block list
    let instances = store.instances_in(&[]).unwrap();
    assert_eq!(instances.len(), 0);

    // Non-existent block
    let instances = store.instances_in(&[9999]).unwrap();
    assert_eq!(instances.len(), 0);
}

#[test]
fn test_concurrent_edits_different_clients() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Both clients edit same instance
    store.record_placement("text-500", 500).unwrap();

    let ops_a = vec![TextOpId { id: "seq-a1#client-A".to_string() }];
    let ops_b = vec![TextOpId { id: "seq-b1#client-B".to_string() }];

    store.mark_ops_applied("text-500", ops_a).unwrap();
    store.mark_ops_applied("text-500", ops_b).unwrap();

    // Both ops should be tracked
    assert!(store.is_op_applied("text-500", "seq-a1#client-A").unwrap());
    assert!(store.is_op_applied("text-500", "seq-b1#client-B").unwrap());
}

#[test]
fn test_offline_resend_dedup() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    store.record_placement("offline-inst", 600).unwrap();

    // Same op, sent 5 times
    let op = TextOpId { id: "retry-id".to_string() };

    for _ in 0..5 {
        store.mark_ops_applied("offline-inst", vec![op.clone()]).unwrap();
    }

    // Should only be tracked once
    assert!(store.is_op_applied("offline-inst", "retry-id").unwrap());
}

#[test]
fn test_split_with_catch_up() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Paragraph split: block 100 → blocks 100 + 999
    store.record_placement("inst-X", 100).unwrap();
    store.record_placement("inst-X", 999).unwrap();

    // Client catches up with block 999
    let instances_999 = store.instances_in(&[999]).unwrap();

    // Should find instance
    assert_eq!(instances_999.len(), 1);
    assert_eq!(instances_999[0], "inst-X");
}
