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
