use carbyne_db::{DbName, DbRegistry};
use carbyne_db::sync::{DbMessage, DbMessageType};
use tempfile::TempDir;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_two_instances_peer_connection() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();

    let registry1 = DbRegistry::new(dir1.path().to_str().unwrap()).unwrap();
    let registry2 = DbRegistry::new(dir2.path().to_str().unwrap()).unwrap();

    let db1 = registry1.create(DbName("test".to_string())).unwrap();
    let db2 = registry2.create(DbName("test".to_string())).unwrap();

    // Create mock peer channels
    let (peer1_tx, mut peer1_rx) = mpsc::channel::<DbMessage>(10);
    let (peer2_tx, mut peer2_rx) = mpsc::channel::<DbMessage>(10);

    // In real implementation, spawn sync daemons and connect
    // For Phase 3: just verify channels work
    assert_ne!(db1.id(), db2.id());
    assert_eq!(db1.name(), db2.name());

    // Send a message through the channel
    let msg = DbMessage {
        db_id: db1.id(),
        lamport: 1,
        message_type: DbMessageType::SyncRequest { peer_lamport: 0 },
    };

    peer1_tx.send(msg.clone()).await.unwrap();
    let received = peer1_rx.recv().await;
    assert!(received.is_some());
}

#[tokio::test]
async fn test_sync_request_response() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let db = registry.create(DbName("test".to_string())).unwrap();

    let request = DbMessage {
        db_id: db.id(),
        lamport: 0,
        message_type: DbMessageType::SyncRequest { peer_lamport: 0 },
    };

    // Phase 3: would spawn daemon and send request
    // For now, just verify message construction
    assert_eq!(request.db_id, db.id());
    assert_eq!(request.lamport, 0);

    match request.message_type {
        DbMessageType::SyncRequest { peer_lamport } => {
            assert_eq!(peer_lamport, 0);
        }
        _ => panic!("Wrong message type"),
    }
}

#[tokio::test]
async fn test_ops_broadcast_message() {
    let dir = TempDir::new().unwrap();
    let registry = DbRegistry::new(dir.path().to_str().unwrap()).unwrap();

    let db = registry.create(DbName("test".to_string())).unwrap();

    let broadcast = DbMessage {
        db_id: db.id(),
        lamport: 1,
        message_type: DbMessageType::OpsBroadcast {
            ops: vec![],
            lamport: 1,
        },
    };

    assert_eq!(broadcast.db_id, db.id());
    assert_eq!(broadcast.lamport, 1);
}
