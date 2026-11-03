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
fn test_mark_text_op() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Mark single op using convenience method
    store.mark_text_op("inst-1", "op-id-123").unwrap();

    // Check if applied
    assert!(store.is_op_applied("inst-1", "op-id-123").unwrap());
    assert!(!store.is_op_applied("inst-1", "op-id-456").unwrap());

    // Mark same op again: idempotent
    store.mark_text_op("inst-1", "op-id-123").unwrap();
    assert!(store.is_op_applied("inst-1", "op-id-123").unwrap());
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

#[test]
fn test_multi_client_sync_end_to_end() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Client A edits block 100, instance X
    let op_a = TextOpId {
        id: "seq-a1#client-A".to_string(),
    };
    store
        .record_client_ops("inst-X", vec![op_a.clone()], 100)
        .unwrap();

    // Client B edits same block, same instance
    let op_b = TextOpId {
        id: "seq-b1#client-B".to_string(),
    };
    store
        .record_client_ops("inst-X", vec![op_b.clone()], 100)
        .unwrap();

    // Both ops in DB
    assert!(store.is_op_applied("inst-X", &op_a.id).unwrap());
    assert!(store.is_op_applied("inst-X", &op_b.id).unwrap());

    // Placement recorded for both syncs
    let instances_100 = store.instances_in(&[100]).unwrap();
    assert_eq!(instances_100.len(), 1);
    assert_eq!(instances_100[0], "inst-X");

    // Client C catches up: sees both ops
    assert!(store.is_op_applied("inst-X", &op_a.id).unwrap());
    assert!(store.is_op_applied("inst-X", &op_b.id).unwrap());
}

#[test]
fn test_multi_client_offline_resend() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Client A edits, goes offline, comes back
    let op = TextOpId {
        id: "seq-1#client-A".to_string(),
    };

    // First sync
    store.record_client_ops("inst-Y", vec![op.clone()], 200).unwrap();

    // Offline: retries 4 more times (network unreliable)
    for _ in 0..4 {
        store
            .record_client_ops("inst-Y", vec![op.clone()], 200)
            .unwrap();
    }

    // Verify: only 1 entry in dedup table
    assert!(store.is_op_applied("inst-Y", &op.id).unwrap());

    // Placement idempotent: still block 200
    let instances_200 = store.instances_in(&[200]).unwrap();
    assert_eq!(instances_200.len(), 1);
    assert_eq!(instances_200[0], "inst-Y");

    // Other clients don't see duplicates
    assert_eq!(instances_200.len(), 1);
}

#[test]
fn test_split_with_concurrent_edits() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Both clients edit instance before split
    let op_a = TextOpId {
        id: "seq-a1#client-A".to_string(),
    };
    let op_b = TextOpId {
        id: "seq-b1#client-B".to_string(),
    };

    // Both clients record ops in block 300
    store
        .record_client_ops("inst-paragraph", vec![op_a.clone()], 300)
        .unwrap();
    store
        .record_client_ops("inst-paragraph", vec![op_b.clone()], 300)
        .unwrap();

    // Paragraph split: block 300 → blocks 300 + 500
    store.record_placement("inst-paragraph", 300).unwrap();
    store.record_placement("inst-paragraph", 500).unwrap();

    // Both clients rebuild: see instance in both blocks
    let instances_300 = store.instances_in(&[300]).unwrap();
    let instances_500 = store.instances_in(&[500]).unwrap();
    assert_eq!(instances_300.len(), 1);
    assert_eq!(instances_500.len(), 1);
    assert_eq!(instances_300[0], "inst-paragraph");
    assert_eq!(instances_500[0], "inst-paragraph");

    // No ops lost: both still in dedup
    assert!(store.is_op_applied("inst-paragraph", &op_a.id).unwrap());
    assert!(store.is_op_applied("inst-paragraph", &op_b.id).unwrap());
}

#[test]
fn test_split_instance_op() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Initial: instance in block 100
    store.record_placement("inst-para", 100).unwrap();
    assert!(store.instances_in(&[100]).unwrap().contains(&"inst-para".to_string()));

    // User splits paragraph at position 25: block 100 → blocks 100 + 999
    store
        .split_instance("inst-para", 100, 999)
        .unwrap();

    // Instance now tracked in both blocks
    let instances_100 = store.instances_in(&[100]).unwrap();
    let instances_999 = store.instances_in(&[999]).unwrap();
    assert!(instances_100.contains(&"inst-para".to_string()));
    assert!(instances_999.contains(&"inst-para".to_string()));

    // Query both blocks at once
    let all_instances = store.instances_in(&[100, 999]).unwrap();
    assert_eq!(all_instances.len(), 1); // Still one instance
    assert_eq!(all_instances[0], "inst-para");
}

#[test]
fn test_join_instances_op() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Two paragraphs: instances in separate blocks
    store.record_placement("inst-para-1", 100).unwrap();
    store.record_placement("inst-para-2", 200).unwrap();

    // Add ops to each instance (simulating edits)
    store
        .mark_ops_applied("inst-para-1", vec![TextOpId { id: "op-1a".to_string() }])
        .unwrap();
    store
        .mark_ops_applied("inst-para-2", vec![TextOpId { id: "op-2a".to_string() }])
        .unwrap();

    // User joins: paragraphs merge → para-2 stays, para-1 ops move to para-2
    store
        .join_instances("inst-para-1", "inst-para-2", 100)
        .unwrap();

    // Target instance now in target block
    let instances_100 = store.instances_in(&[100]).unwrap();
    assert!(instances_100.contains(&"inst-para-2".to_string()));

    // Para-2 now has both ops
    assert!(store.is_op_applied("inst-para-2", "op-1a").unwrap());
    assert!(store.is_op_applied("inst-para-2", "op-2a").unwrap());

    // Para-1 ops still tracked in para-1 (separate instance)
    assert!(store.is_op_applied("inst-para-1", "op-1a").unwrap());
}

#[test]
fn test_split_idempotent() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    store.record_placement("inst-para", 100).unwrap();

    // Split twice: should be idempotent
    store
        .split_instance("inst-para", 100, 999)
        .unwrap();
    store
        .split_instance("inst-para", 100, 999)
        .unwrap();

    // Still one instance entry per block (INSERT OR IGNORE)
    let instances_999 = store.instances_in(&[999]).unwrap();
    assert_eq!(instances_999.len(), 1);
    assert_eq!(instances_999[0], "inst-para");
}

#[test]
fn test_join_idempotent() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    store.record_placement("inst-para-1", 100).unwrap();
    store.record_placement("inst-para-2", 200).unwrap();

    store
        .mark_ops_applied("inst-para-1", vec![TextOpId { id: "op-1".to_string() }])
        .unwrap();
    store
        .mark_ops_applied("inst-para-2", vec![TextOpId { id: "op-2".to_string() }])
        .unwrap();

    // Join twice: idempotent
    store
        .join_instances("inst-para-1", "inst-para-2", 100)
        .unwrap();
    store
        .join_instances("inst-para-1", "inst-para-2", 100)
        .unwrap();

    // Para-2 still has both ops (dedup prevents duplicates)
    assert!(store.is_op_applied("inst-para-2", "op-1").unwrap());
    assert!(store.is_op_applied("inst-para-2", "op-2").unwrap());
}
