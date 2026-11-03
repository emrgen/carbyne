use carbyne::{ClientState, Content};
use carbyne_db::sync::{DbMessage, DbMessageType, Inbound, SyncDaemon};
use carbyne_db::{DbInstance, DbName, DbRegistry};
use tempfile::TempDir;
use tokio::sync::mpsc;

fn get(db: &DbInstance, k: &str) -> Option<Content> {
    db.doc().get(k).map(|t| t.content())
}

fn put(db: &DbInstance, k: &str, v: &str) {
    let doc = db.doc();
    doc.set(k, doc.atom(v));
    drop(doc);
    db.commit().unwrap();
}

/// a edits; b sends SyncRequest with its version; a replies SyncResponse;
/// b applies it; b's doc now matches and the diff is on b's tail.
#[tokio::test]
async fn test_sync_request_response_round_trip() {
    let d1 = TempDir::new().unwrap();
    let d2 = TempDir::new().unwrap();
    let r1 = DbRegistry::new(d1.path().to_str().unwrap()).unwrap();
    let r2 = DbRegistry::new(d2.path().to_str().unwrap()).unwrap();
    let a = r1.create(DbName("x".into())).unwrap();
    let b = r2
        .create_from(DbName("x".into()), &a.diff_since(ClientState::default()))
        .unwrap();

    let (daemon_a, _tx_a) = SyncDaemon::new(a.clone());
    let (daemon_b, _tx_b) = SyncDaemon::new(b.clone());
    // b's mailbox, as seen by a
    let (b_tx, mut b_rx) = mpsc::channel::<DbMessage>(8);
    daemon_a.add_peer("b".into(), b_tx);
    assert_eq!(daemon_a.peer_count(), 1);

    put(&a, "k", "v");

    daemon_a
        .handle_message(Inbound {
            from: "b".into(),
            msg: DbMessage {
                db_id: b.id(),
                version: b.version().to_bytes(),
                message_type: DbMessageType::SyncRequest {
                    since: b.version().to_bytes(),
                },
            },
        })
        .await
        .unwrap();

    let reply = b_rx.recv().await.unwrap();
    assert!(matches!(reply.message_type, DbMessageType::SyncResponse { .. }));
    daemon_b
        .handle_message(Inbound {
            from: "a".into(),
            msg: reply,
        })
        .await
        .unwrap();

    assert_eq!(get(&b, "k"), Some(Content::String("v".into())));
    assert_eq!(b.seq(), 1);
    assert!(a.diff_since(b.version()).is_empty());
}

#[tokio::test]
async fn test_malformed_payload_is_error_not_panic() {
    let d = TempDir::new().unwrap();
    let r = DbRegistry::new(d.path().to_str().unwrap()).unwrap();
    let a = r.create(DbName("x".into())).unwrap();
    let (daemon, _tx) = SyncDaemon::new(a.clone());
    let res = daemon
        .handle_message(Inbound {
            from: "p".into(),
            msg: DbMessage {
                db_id: a.id(),
                version: vec![],
                message_type: DbMessageType::OpsBroadcast { diff: vec![1, 2, 3] },
            },
        })
        .await;
    assert!(res.is_err());
    assert_eq!(a.seq(), 0, "nothing persisted");
}
