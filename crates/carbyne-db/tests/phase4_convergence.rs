/// Phase 4: Op merge via Diff
/// Tests that diffs can be applied and lamport clock tracks merges.

use carbyne_db::{DbInstance, DbName, DbRegistry};
use tempfile::TempDir;

#[test]
fn test_apply_increments_lamport() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("test".to_string())).unwrap();
    let db2 = registry2.create(DbName("test".to_string())).unwrap();

    // Initial lamports: both 0
    assert_eq!(db1.lamport(), 0);
    assert_eq!(db2.lamport(), 0);

    // Create a diff (any diff, just for testing)
    let diff = {
        let doc1 = db1.doc();
        doc1.set("test", doc1.string("value"));
        carbyne::ClientState::default()
    };

    // Create a diff manually to apply to db2
    let test_diff = {
        let doc1 = db1.doc();
        doc1.diff(diff)
    };

    // db2 applies 1 diff
    db2.apply(vec![test_diff]).unwrap();
    assert_eq!(db2.lamport(), 1);

    // db2 applies 2 more diffs
    let test_diff2 = {
        let doc1 = db1.doc();
        doc1.diff(carbyne::ClientState::default())
    };
    db2.apply(vec![test_diff2]).unwrap();
    assert_eq!(db2.lamport(), 2);
}

#[test]
fn test_multiple_diffs_in_one_apply() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();
    let db = registry.create(DbName("test".to_string())).unwrap();

    assert_eq!(db.lamport(), 0);

    // Create multiple dummy diffs
    let diffs = vec![
        {
            let doc = db.doc();
            doc.set("a", doc.string("1"));
            doc.diff(carbyne::ClientState::default())
        },
        {
            let doc = db.doc();
            doc.set("b", doc.string("2"));
            doc.diff(carbyne::ClientState::default())
        },
        {
            let doc = db.doc();
            doc.set("c", doc.string("3"));
            doc.diff(carbyne::ClientState::default())
        },
    ];

    // Apply all 3 at once
    db.apply(diffs).unwrap();

    // Lamport should be 3
    assert_eq!(db.lamport(), 3);
}

#[test]
fn test_version_tracking() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();
    let db = registry.create(DbName("test".to_string())).unwrap();

    let v1 = db.version();

    // Make a change
    {
        let doc = db.doc();
        doc.set("key", doc.string("value"));
    }

    let v2 = db.version();

    // Version should be different
    assert_ne!(v1, v2);
}

#[test]
fn test_instance_create_and_apply() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();

    // Create two instances
    let db1 = registry1.create(DbName("shared".to_string())).unwrap();
    let db2 = registry2.create(DbName("shared".to_string())).unwrap();

    // Both should exist and have lamport 0
    assert_eq!(db1.lamport(), 0);
    assert_eq!(db2.lamport(), 0);
    assert_eq!(db1.name(), &DbName("shared".to_string()));
    assert_eq!(db2.name(), &DbName("shared".to_string()));

    // Simulate receiving empty op batch (should not error)
    db1.apply(vec![]).unwrap();
    assert_eq!(db1.lamport(), 0);
}
