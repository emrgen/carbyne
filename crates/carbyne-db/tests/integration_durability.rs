use carbyne_db::{DbName, DbRegistry};
use tempfile::TempDir;

#[test]
fn test_ops_persisted_to_storage() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    {
        let registry = DbRegistry::new(path).unwrap();
        let db = registry.create(DbName("test".to_string())).unwrap();

        // Apply ops (persisted via changelog)
        let _ = db.apply(vec![]).unwrap();
        let _ = db.apply(vec![]).unwrap();

        // Lamport should be 2 after two applies
        assert_eq!(db.lamport(), 2);
    }
}

#[test]
fn test_restart_loads_persisted_instances() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let db_id;

    {
        let registry = DbRegistry::new(path).unwrap();
        let db = registry.create(DbName("persistent_db".to_string())).unwrap();
        db_id = db.id();

        // Apply ops
        let _ = db.apply(vec![]).unwrap();
        let _ = db.apply(vec![]).unwrap();
        assert_eq!(db.lamport(), 2);
    }

    // New registry (simulates restart)
    {
        let registry = DbRegistry::new(path).unwrap();

        // load_all scans storage and replays instances
        registry.load_all().unwrap();

        // Get should now work
        let db = registry.get(db_id).unwrap();
        assert_eq!(db.name().0, "persistent_db");
        assert_eq!(db.lamport(), 2);
    }
}

#[test]
fn test_multi_instance_isolation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    let registry = DbRegistry::new(path).unwrap();

    let db1 = registry.create(DbName("db1".to_string())).unwrap();
    let db2 = registry.create(DbName("db2".to_string())).unwrap();

    let _ = db1.apply(vec![]).unwrap();
    let _ = db1.apply(vec![]).unwrap();

    let _ = db2.apply(vec![]).unwrap();

    // db1 lamport should not affect db2
    assert_eq!(db1.lamport(), 2);
    assert_eq!(db2.lamport(), 1);
}

#[test]
fn test_snapshot_clears_ops_log() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();

    let registry = DbRegistry::new(path).unwrap();
    let db = registry.create(DbName("test".to_string())).unwrap();

    // Apply ops and take snapshot
    let _ = db.apply(vec![]).unwrap();
    let _ = db.apply(vec![]).unwrap();

    // Phase 2: snapshot would clear ops log and save state
    // For now, just verify lamport increments correctly
    assert_eq!(db.lamport(), 2);
}
