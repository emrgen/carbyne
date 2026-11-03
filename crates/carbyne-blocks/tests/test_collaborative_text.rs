use carbyne::Doc;
use carbyne_blocks::BlockStore;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn test_append_and_record_placement() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // 1. Record that instance-1 is in block 123
    store.record_placement("inst-1", 123).unwrap();

    // 2. Query: which instances in block 123?
    let instances = store.instances_in(&[123]).unwrap();
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0], "inst-1");
}

#[test]
fn test_split_scenario() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Scenario: paragraph in block 123 is split → new block 999
    // Both blocks have same text instance (instance-A)

    // Record placements
    store.record_placement("inst-A", 123).unwrap();
    store.record_placement("inst-A", 999).unwrap();

    // Query: which instances in block 999?
    let instances_999 = store.instances_in(&[999]).unwrap();
    assert_eq!(instances_999.len(), 1);
    assert_eq!(instances_999[0], "inst-A");

    // Query: which instances in block 123?
    let instances_123 = store.instances_in(&[123]).unwrap();
    assert_eq!(instances_123.len(), 1);

    // Query: which instances in BOTH blocks?
    let instances_both = store.instances_in(&[123, 999]).unwrap();
    assert_eq!(instances_both.len(), 1); // Same instance
}

#[test]
fn test_multiple_instances_in_one_block() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Block 123 has multiple instances (e.g., two separate text fields)
    store.record_placement("inst-title", 123).unwrap();
    store.record_placement("inst-content", 123).unwrap();

    // Query: which instances?
    let instances = store.instances_in(&[123]).unwrap();
    assert_eq!(instances.len(), 2);
    assert!(instances.contains(&"inst-title".to_string()));
    assert!(instances.contains(&"inst-content".to_string()));
}

#[test]
fn test_instance_idempotence() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Record same placement multiple times (INSERT OR IGNORE)
    store.record_placement("inst-x", 456).unwrap();
    store.record_placement("inst-x", 456).unwrap();
    store.record_placement("inst-x", 456).unwrap();

    // Should only have one entry
    let instances = store.instances_in(&[456]).unwrap();
    assert_eq!(instances.len(), 1);
}

#[test]
fn test_get_text_ops_since() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // No instances = no ops
    let ops = store.get_text_ops_since(&[], &vec![1]).unwrap();
    assert_eq!(ops.len(), 0);

    // Empty instance list = no ops
    let ops = store.get_text_ops_since(&vec![], &vec![1]).unwrap();
    assert_eq!(ops.len(), 0);
}

#[test]
fn test_empty_queries() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Query with no block IDs
    let instances = store.instances_in(&[]).unwrap();
    assert_eq!(instances.len(), 0);

    // Query with no instances
    let ops = store.get_text_ops_since(&vec![], &vec![0]).unwrap();
    assert_eq!(ops.len(), 0);
}

#[test]
fn test_concurrent_edits_different_clients() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Scenario: Client-A and Client-B both edit block 500
    // Their ops should both append (different IDs)
    
    store.record_placement("text-500", 500).unwrap();

    let ops_a = vec![
        carbyne_blocks::CrdtTextOp {
            seq: 1,
            block_id: 500,
            instance_id: "text-500".to_string(),
            kind: "insert".to_string(),
            id_seq: "seq-a1".to_string(),
            id_site: "client-A".to_string(),
            payload: b"a".to_vec(),
            applied_version: vec![1],
        },
    ];

    let ops_b = vec![
        carbyne_blocks::CrdtTextOp {
            seq: 0,
            block_id: 500,
            instance_id: "text-500".to_string(),
            kind: "insert".to_string(),
            id_seq: "seq-b1".to_string(),
            id_site: "client-B".to_string(),
            payload: b"b".to_vec(),
            applied_version: vec![1],
        },
    ];

    // Both should append
    store.append_text_ops(500, ops_a).unwrap();
    store.append_text_ops(500, ops_b).unwrap();

    // Verify both are in log
    let instances = store.instances_in(&[500]).unwrap();
    assert_eq!(instances.len(), 1);

    let ops = store.get_text_ops_since(&instances, &vec![0]).unwrap();
    assert_eq!(ops.len(), 2);
    assert_ne!(ops[0].id_site, ops[1].id_site);
}

#[test]
fn test_offline_resend_idempotent() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    store.record_placement("offline-inst", 600).unwrap();

    // Same op, sent 5 times (network unreliable)
    let op = carbyne_blocks::CrdtTextOp {
        seq: 0,
        block_id: 600,
        instance_id: "offline-inst".to_string(),
        kind: "insert".to_string(),
        id_seq: "retry-id".to_string(),
        id_site: "offline-client".to_string(),
        payload: b"typed while offline".to_vec(),
        applied_version: vec![1],
    };

    // Send 5 times
    for _ in 0..5 {
        store.append_text_ops(600, vec![op.clone()]).unwrap();
    }

    // Should have exactly 1 op (UNIQUE dedup)
    let instances = store.instances_in(&[600]).unwrap();
    let ops = store.get_text_ops_since(&instances, &vec![0]).unwrap();
    assert_eq!(ops.len(), 1);
}

#[test]
fn test_split_with_catch_up() {
    let doc = Arc::new(Doc::new(Default::default()));
    let tmpfile = NamedTempFile::new().unwrap();
    let store = BlockStore::new(doc.clone(), tmpfile.path()).unwrap();

    // Paragraph text exists in block 100, instance-X
    store.record_placement("inst-X", 100).unwrap();

    // User splits: text now in blocks 100 and 999 (same instance)
    store.record_placement("inst-X", 999).unwrap();

    // Client C catches up with just block 999
    let instances_999 = store.instances_in(&[999]).unwrap();

    // Should find instance-X
    assert_eq!(instances_999.len(), 1);
    assert_eq!(instances_999[0], "inst-X");

    // If there were ops for inst-X from block 100, they'd also be found
    // This is the key: instance tracking survives split
}
