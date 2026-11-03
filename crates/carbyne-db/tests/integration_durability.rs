use carbyne::{ClientState, Content, Diff};
use carbyne_db::{DbInstance, DbName, DbRegistry};
use tempfile::TempDir;

fn get(db: &DbInstance, k: &str) -> Option<Content> {
    db.doc().get(k).map(|t| t.content())
}

fn put(db: &DbInstance, k: &str, v: &str) {
    let doc = db.doc();
    doc.set(k, doc.atom(v));
    drop(doc);
    db.commit().unwrap();
}

#[test]
fn test_commit_appends_and_empty_commit_is_noop() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();
    let db = registry.create(DbName("test".to_string())).unwrap();

    db.commit().unwrap();
    assert_eq!(db.seq(), 0, "nothing to persist");

    put(&db, "a", "1");
    put(&db, "b", "2");
    assert_eq!(db.seq(), 2);
    assert_eq!(db.changelog().tail_len().unwrap(), 2);
}

#[test]
fn test_restart_replays_tail() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let db_id;
    let before;

    {
        let registry = DbRegistry::new(path).unwrap();
        let db = registry.create(DbName("persistent_db".to_string())).unwrap();
        db_id = db.id();
        put(&db, "k", "v");
        put(&db, "k2", "v2");
        before = get(&db, "k");
    }

    let registry = DbRegistry::new(path).unwrap();
    registry.load_all().unwrap();
    let db = registry.get(db_id).unwrap();
    assert_eq!(db.name().0, "persistent_db");
    assert_eq!(db.seq(), 2);
    assert_eq!(get(&db, "k"), before);
    assert_eq!(get(&db, "k2"), Some(Content::String("v2".into())));
}

#[test]
fn test_snapshot_compacts_tail_and_restart_matches() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let db_id;
    let expected: Vec<Option<Content>>;

    {
        let registry = DbRegistry::new(path).unwrap();
        let db = registry.create(DbName("snap".to_string())).unwrap();
        db_id = db.id();
        for i in 0..70 {
            put(&db, &format!("k{i}"), &i.to_string());
        }
        assert_eq!(db.seq(), 70);
        // snapshot at seq 64 wiped tail ≤64; only 65..70 remain
        assert_eq!(db.changelog().tail_len().unwrap(), 6);
        expected = (0..70).map(|i| get(&db, &format!("k{i}"))).collect();
    }

    let registry = DbRegistry::new(path).unwrap();
    registry.load_all().unwrap();
    let db = registry.get(db_id).unwrap();
    assert_eq!(db.seq(), 70);
    let actual: Vec<Option<Content>> = (0..70).map(|i| get(&db, &format!("k{i}"))).collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_apply_remote_diff_persists_and_syncs() {
    let d1 = TempDir::new().unwrap();
    let d2 = TempDir::new().unwrap();
    let r1 = DbRegistry::new(d1.path().to_str().unwrap()).unwrap();
    let r2 = DbRegistry::new(d2.path().to_str().unwrap()).unwrap();
    let a = r1.create(DbName("x".to_string())).unwrap();
    put(&a, "seed", "0");
    // b bootstraps from a's full state (same doc identity), then syncs increments
    let b = r2.create_from(DbName("x".to_string()), &a.diff_since(ClientState::default()))
        .unwrap();
    assert_eq!(get(&b, "seed"), Some(Content::String("0".into())));

    put(&a, "from_a", "1");
    let bytes = a.diff_since(b.version()).to_bytes();
    b.apply(&Diff::from_bytes(&bytes).unwrap()).unwrap();

    assert_eq!(get(&b, "from_a"), Some(Content::String("1".into())));
    assert_eq!(b.seq(), 1);
    assert!(a.diff_since(b.version()).is_empty());
}

#[test]
fn test_multi_instance_isolation() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();
    let db1 = registry.create(DbName("db1".to_string())).unwrap();
    let db2 = registry.create(DbName("db2".to_string())).unwrap();

    put(&db1, "only1", "x");
    assert_eq!(db1.seq(), 1);
    assert_eq!(db2.seq(), 0);
    assert_eq!(get(&db2, "only1"), None);
}
