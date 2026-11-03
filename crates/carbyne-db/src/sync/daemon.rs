use crate::instance::{DbInstance, Op};
use crate::sync::message::{DbMessage, DbMessageType};
use crate::sync::peer::PeerTx;
use carbyne::ClientState;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub type PeerId = String;

/// Inbound message tagged with the sending peer so replies go back to it.
pub struct Inbound {
    pub from: PeerId,
    pub msg: DbMessage,
}

pub struct SyncDaemon {
    instance: Arc<DbInstance>,
    peers: Arc<DashMap<PeerId, PeerTx>>,
    rx: Receiver<Inbound>,
}

impl SyncDaemon {
    pub fn new(instance: Arc<DbInstance>) -> (Self, Sender<Inbound>) {
        let (tx, rx) = mpsc::channel(100);

        (
            Self {
                instance,
                peers: Arc::new(DashMap::new()),
                rx,
            },
            tx,
        )
    }

    pub fn add_peer(&self, peer_id: PeerId, tx: PeerTx) {
        self.peers.insert(peer_id, tx);
    }

    pub fn remove_peer(&self, peer_id: &str) {
        self.peers.remove(peer_id);
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub async fn run(&mut self) {
        while let Some(inbound) = self.rx.recv().await {
            let _ = self.handle_message(inbound).await;
        }
    }

    fn envelope(&self, message_type: DbMessageType) -> DbMessage {
        DbMessage {
            db_id: self.instance.id(),
            lamport: self.instance.lamport(),
            message_type,
        }
    }

    /// Handle one message. Err on malformed payload or storage failure;
    /// `run` drops it so a bad peer cannot stop the loop.
    pub async fn handle_message(&self, inbound: Inbound) -> Result<(), String> {
        let Inbound { from, msg } = inbound;
        if msg.db_id != self.instance.id() {
            return Err("db_id mismatch".into());
        }

        match msg.message_type {
            DbMessageType::SyncRequest { since } => {
                // Peer asks for ops after version `since`
                let since = ClientState::from_bytes(&since)
                    .map_err(|e| format!("parse since: {}", e))?;
                let ops = self.instance.ops_since(since);

                // Phase 3.5: Serialize ops to bytes for transmission
                let ops_bytes = serialize_ops(&ops);
                if let Some(peer) = self.peers.get(&from) {
                    let _ = peer
                        .send(self.envelope(DbMessageType::SyncResponse {
                            ops: ops_bytes,
                        }))
                        .await;
                }
            }
            DbMessageType::SyncResponse { ops: ops_bytes }
            | DbMessageType::OpsBroadcast { ops: ops_bytes } => {
                // Receive ops from peer, merge into local doc
                let ops = deserialize_ops(&ops_bytes)
                    .map_err(|e| format!("deserialize ops: {}", e))?;
                self.instance
                    .apply(ops.clone())
                    .map_err(|e| format!("apply ops: {}", e))?;

                // Fan-out to every peer except the sender (gossip)
                let out = self.envelope(DbMessageType::OpsBroadcast {
                    ops: ops_bytes,
                });
                for peer in self.peers.iter() {
                    if peer.key() != &from {
                        let _ = peer.value().send(out.clone()).await;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Phase 3.5: Placeholder serialization for carbyne::Change ops.
/// TODO: Implement real carbyne change serialization.
fn serialize_ops(_ops: &[Op]) -> Vec<u8> {
    // For now, empty placeholder; real impl uses carbyne codec
    vec![]
}

/// Phase 3.5: Placeholder deserialization for ops.
fn deserialize_ops(_bytes: &[u8]) -> Result<Vec<Op>, String> {
    // For now, return empty; real impl parses carbyne codec
    Ok(vec![])
}
