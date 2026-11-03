use carbyne_db::{DbName, DbRegistry};
use carbyne_db::sync::{DbMessage, DbMessageType, SyncDaemon};
use std::sync::Arc;

#[test]
fn test_two_instances_separate_storage() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();
    let db1 = registry1.create(DbName("sync_test".to_string())).unwrap();
    let db2 = registry2.create(DbName("sync_test".to_string())).unwrap();

    // Verify separate instances
    assert_ne!(db1.id(), db2.id());
    assert_eq!(db1.name(), db2.name());

    // Apply ops to db1
    let _ = db1.apply(vec![]).unwrap();
    let _ = db1.apply(vec![]).unwrap();
    assert_eq!(db1.lamport(), 2);

    // db2 should start at lamport 0 (separate storage)
    assert_eq!(db2.lamport(), 0);

    // Apply different ops to db2
    let _ = db2.apply(vec![]).unwrap();
    assert_eq!(db2.lamport(), 1);

    // Lamports remain independent (not synced yet)
    assert_eq!(db1.lamport(), 2);
    assert_eq!(db2.lamport(), 1);
}

#[test]
fn test_sync_daemon_peer_connections() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();
    let dir3 = tempfile::TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();
    let registry3 = DbRegistry::new(dir3.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("test".to_string())).unwrap();
    let db2 = registry2.create(DbName("test".to_string())).unwrap();
    let db3 = registry3.create(DbName("test".to_string())).unwrap();

    // Create sync daemons
    let (daemon1, tx1) = SyncDaemon::new(db1.clone());
    let (daemon2, tx2) = SyncDaemon::new(db2.clone());
    let (daemon3, tx3) = SyncDaemon::new(db3.clone());

    // Connect peers in mesh
    daemon1.add_peer("peer2".to_string(), tx2.clone());
    daemon1.add_peer("peer3".to_string(), tx3.clone());

    daemon2.add_peer("peer1".to_string(), tx1.clone());
    daemon2.add_peer("peer3".to_string(), tx3.clone());

    daemon3.add_peer("peer1".to_string(), tx1.clone());
    daemon3.add_peer("peer2".to_string(), tx2.clone());

    // Verify mesh connectivity
    assert_eq!(daemon1.peer_count(), 2);
    assert_eq!(daemon2.peer_count(), 2);
    assert_eq!(daemon3.peer_count(), 2);

    // Remove a peer
    daemon1.remove_peer("peer2");
    assert_eq!(daemon1.peer_count(), 1);

    // Re-add it
    daemon1.add_peer("peer2".to_string(), tx2);
    assert_eq!(daemon1.peer_count(), 2);
}

#[tokio::test]
async fn test_sync_message_queue() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("queue_test".to_string())).unwrap();
    let db2 = registry2.create(DbName("queue_test".to_string())).unwrap();

    let (daemon1, tx1) = SyncDaemon::new(db1.clone());
    let (_daemon2, tx2) = SyncDaemon::new(db2.clone());

    daemon1.add_peer("peer2".to_string(), tx2.clone());

    // Send a sync request from db1 to db2
    let request = DbMessage {
        db_id: db1.id(),
        lamport: 1,
        message_type: DbMessageType::SyncRequest { peer_lamport: 0 },
    };

    // Send should succeed (back-pressure via channel buffer)
    assert!(tx2.send(request).await.is_ok());

    // Verify peer still connected
    assert_eq!(daemon1.peer_count(), 1);
}

#[tokio::test]
async fn test_back_pressure_limits() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("backpressure".to_string())).unwrap();
    let db2 = registry2.create(DbName("backpressure".to_string())).unwrap();

    let (_daemon1, _tx1) = SyncDaemon::new(db1.clone());
    let (_daemon2, tx2) = SyncDaemon::new(db2.clone());

    // Channel buffer is 100; fill it
    for i in 0..100 {
        let msg = DbMessage {
            db_id: db1.id(),
            lamport: i,
            message_type: DbMessageType::SyncRequest { peer_lamport: 0 },
        };
        // All should succeed (buffer not full)
        assert!(tx2.try_send(msg).is_ok());
    }

    // 101st should block or fail
    let msg_full = DbMessage {
        db_id: db1.id(),
        lamport: 100,
        message_type: DbMessageType::SyncRequest { peer_lamport: 0 },
    };
    // try_send on full buffer returns Err
    assert!(tx2.try_send(msg_full).is_err());
}

#[test]
fn test_instance_lamport_causality() {
    let dir = tempfile::TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let db = registry.create(DbName("lamport_test".to_string())).unwrap();

    // Lamport starts at 0
    assert_eq!(db.lamport(), 0);

    // Each apply increments
    for i in 1..=5 {
        let _ = db.apply(vec![]).unwrap();
        assert_eq!(db.lamport(), i);
    }

    // Lamport is monotonically increasing
    let lamport1 = db.lamport();
    let _ = db.apply(vec![]).unwrap();
    let lamport2 = db.lamport();
    assert!(lamport2 > lamport1);
}
