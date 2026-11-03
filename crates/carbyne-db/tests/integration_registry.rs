use carbyne_db::{DbId, DbName, DbRegistry};
use tempfile::TempDir;

#[test]
fn test_create_and_get_instance() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let instance = registry.create(DbName("test".to_string())).unwrap();
    let retrieved = registry.get(instance.id()).unwrap();

    assert_eq!(instance.id(), retrieved.id());
    assert_eq!(instance.name(), retrieved.name());
}

#[test]
fn test_list_instances() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let db1 = registry.create(DbName("db1".to_string())).unwrap();
    let db2 = registry.create(DbName("db2".to_string())).unwrap();

    let list = registry.list();
    let ids: Vec<DbId> = list.iter().map(|(id, _)| *id).collect();
    assert!(ids.contains(&db1.id()));
    assert!(ids.contains(&db2.id()));
}

#[test]
fn test_delete_instance() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let instance = registry.create(DbName("test".to_string())).unwrap();
    let id = instance.id();

    registry.delete(id).unwrap();

    assert!(registry.get(id).is_err());
}

#[test]
fn test_instance_not_found() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let instance = registry.create(DbName("test".to_string())).unwrap();
    registry.delete(instance.id()).unwrap();

    let result = registry.get(instance.id());
    assert!(result.is_err());
}
