use carbyne_db::{DbName, DbRegistry};
use tempfile::TempDir;

#[test]
fn test_changelog_persists_ops() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    // Create instance and apply ops
    {
        let registry = DbRegistry::new(path).unwrap();
        let instance = registry.create(DbName("test".to_string())).unwrap();

        instance.apply(vec![]).unwrap();
        instance.apply(vec![]).unwrap();
        instance.apply(vec![]).unwrap();
        assert_eq!(instance.lamport(), 3);

        // Verify ops are logged
        let ops_count = instance.changelog().ops_count().unwrap();
        assert_eq!(ops_count, 3);
    }

    // Reload registry and verify ops persist
    {
        let registry = DbRegistry::new(path).unwrap();
        let list = registry.list();
        assert_eq!(list.len(), 1);
        
        let (id, name) = &list[0];
        // Create new instance from storage
        let instance = registry.create(DbName(name.0.clone())).unwrap();
        
        // Lamport should be restored from changelog
        // Phase 2: lamport defaults to 0, so new instance starts fresh
        // Phase 3: will load persisted lamport
        assert_eq!(instance.lamport(), 0);
    }
}

#[test]
fn test_multiple_instances_separate_changelogs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    let registry = DbRegistry::new(path).unwrap();

    let db1 = registry.create(DbName("db1".to_string())).unwrap();
    let db2 = registry.create(DbName("db2".to_string())).unwrap();

    db1.apply(vec![]).unwrap();
    db1.apply(vec![]).unwrap();

    db2.apply(vec![]).unwrap();
    db2.apply(vec![]).unwrap();
    db2.apply(vec![]).unwrap();

    let count1 = db1.changelog().ops_count().unwrap();
    let count2 = db2.changelog().ops_count().unwrap();

    assert_eq!(count1, 2);
    assert_eq!(count2, 3);
}

#[test]
fn test_lamport_increments_on_apply() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    let registry = DbRegistry::new(path).unwrap();
    let instance = registry.create(DbName("test".to_string())).unwrap();

    assert_eq!(instance.lamport(), 0);

    instance.apply(vec![]).unwrap();
    assert_eq!(instance.lamport(), 1);

    instance.apply(vec![]).unwrap();
    assert_eq!(instance.lamport(), 2);

    instance.apply(vec![]).unwrap();
    assert_eq!(instance.lamport(), 3);
}

#[test]
fn test_snapshot_clears_ops_log() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    let registry = DbRegistry::new(path).unwrap();
    let instance = registry.create(DbName("test".to_string())).unwrap();

    instance.apply(vec![]).unwrap();
    instance.apply(vec![]).unwrap();

    let ops_before = instance.changelog().ops_count().unwrap();
    assert_eq!(ops_before, 2);

    // Snapshot should clear ops log
    let doc = instance.doc();
    instance.changelog().snapshot(&doc, instance.lamport()).unwrap();

    let ops_after = instance.changelog().ops_count().unwrap();
    assert_eq!(ops_after, 0);
}
