use carbyne_db::{DbName, DbRegistry};
use carbyne_db::sync::{DbMessage, DbMessageType, SyncDaemon};
use std::sync::Arc;

#[test]
fn test_instance_offline_and_rejoin() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("offline_test".to_string())).unwrap();
    let db2 = registry2.create(DbName("offline_test".to_string())).unwrap();

    // Create sync daemons
    let (daemon1, tx1) = SyncDaemon::new(db1.clone());
    let (daemon2, tx2) = SyncDaemon::new(db2.clone());

    // Connect peers
    daemon1.add_peer("peer2".to_string(), tx2.clone());
    daemon2.add_peer("peer1".to_string(), tx1.clone());

    assert_eq!(daemon1.peer_count(), 1);
    assert_eq!(daemon2.peer_count(), 1);

    // Simulate peer2 going offline: remove peer1
    daemon2.remove_peer("peer1");
    assert_eq!(daemon2.peer_count(), 0);

    // Verify peer1 still has peer2 (one-way connection down)
    assert_eq!(daemon1.peer_count(), 1);

    // Simulate rejoin: add peer back
    daemon2.add_peer("peer1".to_string(), tx1.clone());
    assert_eq!(daemon2.peer_count(), 1);
}

#[test]
fn test_instance_graceful_leave() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();
    let dir3 = tempfile::TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();
    let registry3 = DbRegistry::new(dir3.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("leave_test".to_string())).unwrap();
    let db2 = registry2.create(DbName("leave_test".to_string())).unwrap();
    let db3 = registry3.create(DbName("leave_test".to_string())).unwrap();

    let (daemon1, tx1) = SyncDaemon::new(db1.clone());
    let (daemon2, tx2) = SyncDaemon::new(db2.clone());
    let (daemon3, tx3) = SyncDaemon::new(db3.clone());

    // Mesh: all connected
    daemon1.add_peer("peer2".to_string(), tx2.clone());
    daemon1.add_peer("peer3".to_string(), tx3.clone());
    daemon2.add_peer("peer1".to_string(), tx1.clone());
    daemon2.add_peer("peer3".to_string(), tx3.clone());
    daemon3.add_peer("peer1".to_string(), tx1.clone());
    daemon3.add_peer("peer2".to_string(), tx2.clone());

    assert_eq!(daemon1.peer_count(), 2);
    assert_eq!(daemon2.peer_count(), 2);
    assert_eq!(daemon3.peer_count(), 2);

    // Instance 2 gracefully leaves: remove all peers
    daemon2.remove_peer("peer1");
    daemon2.remove_peer("peer3");
    assert_eq!(daemon2.peer_count(), 0);

    // Others still have peer2
    assert_eq!(daemon1.peer_count(), 2);
    assert_eq!(daemon3.peer_count(), 2);

    // Others remove peer2
    daemon1.remove_peer("peer2");
    daemon3.remove_peer("peer2");

    assert_eq!(daemon1.peer_count(), 1);
    assert_eq!(daemon3.peer_count(), 1);
}

#[test]
fn test_instance_restart_state_preserved() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let path1 = dir1.path().to_str().unwrap();

    let db_id;
    let lamport_before;

    {
        let registry1 = DbRegistry::new(path1).unwrap();
        let db1 = registry1.create(DbName("restart_test".to_string())).unwrap();
        db_id = db1.id();

        // Apply ops before shutdown
        let _ = db1.apply(vec![]).unwrap();
        let _ = db1.apply(vec![]).unwrap();
        let _ = db1.apply(vec![]).unwrap();
        lamport_before = db1.lamport();

        assert_eq!(lamport_before, 3);
    }

    // Instance shut down and restarted
    {
        let registry1 = DbRegistry::new(path1).unwrap();
        registry1.load_all().unwrap();

        let db1 = registry1.get(db_id).unwrap();
        let lamport_after = db1.lamport();

        // Lamport preserved across restart
        assert_eq!(lamport_after, lamport_before);
        assert_eq!(lamport_after, 3);
    }
}

#[test]
fn test_instance_offline_then_restart() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();
    let path1 = dir1.path().to_str().unwrap();
    let path2 = dir2.path().to_str().unwrap();

    let db1_id;
    let db2_id;

    {
        let registry1 = DbRegistry::new(path1).unwrap();
        let registry2 = DbRegistry::new(path2).unwrap();

        let db1 = registry1.create(DbName("offline_restart".to_string())).unwrap();
        let db2 = registry2.create(DbName("offline_restart".to_string())).unwrap();
        db1_id = db1.id();
        db2_id = db2.id();

        let (daemon1, tx1) = SyncDaemon::new(db1.clone());
        let (daemon2, tx2) = SyncDaemon::new(db2.clone());

        // Connect
        daemon1.add_peer("peer2".to_string(), tx2.clone());
        daemon2.add_peer("peer1".to_string(), tx1.clone());

        // Apply ops to both
        let _ = db1.apply(vec![]).unwrap();
        let _ = db2.apply(vec![]).unwrap();

        // peer2 goes offline
        daemon2.remove_peer("peer1");
        assert_eq!(db2.lamport(), 1);
    }

    // Both instances restart separately
    {
        let registry1 = DbRegistry::new(path1).unwrap();
        registry1.load_all().unwrap();
        let db1_restarted = registry1.get(db1_id).unwrap();
        assert_eq!(db1_restarted.lamport(), 1);

        let registry2 = DbRegistry::new(path2).unwrap();
        registry2.load_all().unwrap();
        let db2_restarted = registry2.get(db2_id).unwrap();
        assert_eq!(db2_restarted.lamport(), 1);

        // Verify peers are gone (not persisted)
        let (_daemon1_new, _tx1_new) = SyncDaemon::new(db1_restarted.clone());
        let (_daemon2_new, _tx2_new) = SyncDaemon::new(db2_restarted);

        // New daemons start with 0 peers (peers not persisted)
        assert_eq!(_daemon1_new.peer_count(), 0);
        assert_eq!(_daemon2_new.peer_count(), 0);
    }
}

#[test]
fn test_multiple_restarts_monotonic_lamport() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_str().unwrap();
    let db_id;

    {
        let registry = DbRegistry::new(path).unwrap();
        let db = registry.create(DbName("multi_restart".to_string())).unwrap();
        db_id = db.id();
        let _ = db.apply(vec![]).unwrap();
        assert_eq!(db.lamport(), 1);
    }

    // Restart 1
    {
        let registry = DbRegistry::new(path).unwrap();
        registry.load_all().unwrap();
        let db = registry.get(db_id).unwrap();
        assert_eq!(db.lamport(), 1);
        let _ = db.apply(vec![]).unwrap();
        assert_eq!(db.lamport(), 2);
    }

    // Restart 2
    {
        let registry = DbRegistry::new(path).unwrap();
        registry.load_all().unwrap();
        let db = registry.get(db_id).unwrap();
        assert_eq!(db.lamport(), 2);
        let _ = db.apply(vec![]).unwrap();
        let _ = db.apply(vec![]).unwrap();
        assert_eq!(db.lamport(), 4);
    }

    // Restart 3
    {
        let registry = DbRegistry::new(path).unwrap();
        registry.load_all().unwrap();
        let db = registry.get(db_id).unwrap();
        assert_eq!(db.lamport(), 4);
    }
}

#[test]
fn test_peer_reconnect_after_restart() {
    let dir1 = tempfile::TempDir::new().unwrap();
    let dir2 = tempfile::TempDir::new().unwrap();
    let path1 = dir1.path().to_str().unwrap();
    let path2 = dir2.path().to_str().unwrap();

    let db1_id;
    let db2_id;

    {
        let registry1 = DbRegistry::new(path1).unwrap();
        let registry2 = DbRegistry::new(path2).unwrap();

        let db1 = registry1.create(DbName("reconnect_test".to_string())).unwrap();
        let db2 = registry2.create(DbName("reconnect_test".to_string())).unwrap();
        db1_id = db1.id();
        db2_id = db2.id();

        let (daemon1, tx1) = SyncDaemon::new(db1.clone());
        let (daemon2, tx2) = SyncDaemon::new(db2.clone());

        daemon1.add_peer("peer2".to_string(), tx2.clone());
        daemon2.add_peer("peer1".to_string(), tx1.clone());

        let _ = db1.apply(vec![]).unwrap();
        let _ = db2.apply(vec![]).unwrap();
    }

    // After restart, peers are not persisted, but can be reconnected
    {
        let registry1 = DbRegistry::new(path1).unwrap();
        let registry2 = DbRegistry::new(path2).unwrap();
        registry1.load_all().unwrap();
        registry2.load_all().unwrap();

        let db1 = registry1.get(db1_id).unwrap();
        let db2 = registry2.get(db2_id).unwrap();

        // State preserved
        assert_eq!(db1.lamport(), 1);
        assert_eq!(db2.lamport(), 1);

        // Re-establish peer connection
        let (daemon1_new, tx1_new) = SyncDaemon::new(db1.clone());
        let (daemon2_new, tx2_new) = SyncDaemon::new(db2.clone());

        daemon1_new.add_peer("peer2".to_string(), tx2_new.clone());
        daemon2_new.add_peer("peer1".to_string(), tx1_new.clone());

        assert_eq!(daemon1_new.peer_count(), 1);
        assert_eq!(daemon2_new.peer_count(), 1);
    }
}

#[test]
fn test_instance_stays_alive_when_peer_removed() {
    let dir = tempfile::TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let db = registry.create(DbName("alive_test".to_string())).unwrap();
    let (_daemon, tx) = SyncDaemon::new(db.clone());

    // Create another daemon just for the peer tx
    let db_peer = registry.create(DbName("peer_db".to_string())).unwrap();
    let (_daemon_peer, tx_peer) = SyncDaemon::new(db_peer);

    // Add peer
    _daemon.add_peer("peer".to_string(), tx_peer);
    assert_eq!(_daemon.peer_count(), 1);

    // Apply ops to original instance
    let _ = db.apply(vec![]).unwrap();
    assert_eq!(db.lamport(), 1);

    // Remove peer—instance should still be alive
    _daemon.remove_peer("peer");
    assert_eq!(_daemon.peer_count(), 0);

    // Instance continues working
    let _ = db.apply(vec![]).unwrap();
    assert_eq!(db.lamport(), 2);
}
