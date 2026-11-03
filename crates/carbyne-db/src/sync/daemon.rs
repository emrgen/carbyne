use crate::instance::DbInstance;
use crate::sync::message::{DbMessage, DbMessageType};
use crate::sync::peer::PeerTx;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};

pub type PeerId = String;

pub struct SyncDaemon {
    instance: Arc<DbInstance>,
    peers: Arc<DashMap<PeerId, PeerTx>>,
    rx: Receiver<DbMessage>,
}

impl SyncDaemon {
    pub fn new(instance: Arc<DbInstance>) -> (Self, Sender<DbMessage>) {
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
        while let Some(msg) = self.rx.recv().await {
            self.handle_message(msg).await;
        }
    }

    async fn handle_message(&self, msg: DbMessage) {
        match msg.message_type {
            DbMessageType::SyncRequest { peer_lamport } => {
                // Phase 3.5: load ops > peer_lamport from changelog
                let ops = vec![];
                let current_lamport = self.instance.lamport();

                let response = DbMessage {
                    db_id: self.instance.id(),
                    lamport: current_lamport,
                    message_type: DbMessageType::SyncResponse {
                        ops,
                        current_lamport,
                    },
                };

                // Broadcast to all connected peers
                for peer in self.peers.iter() {
                    let _ = peer.value().send(response.clone()).await;
                }
            }
            DbMessageType::OpsBroadcast { ops: _, lamport: _ } => {
                // Phase 3.5: merge ops via CRDT, update lamport, broadcast
                // For Phase 3: placeholder
            }
            DbMessageType::SyncResponse { .. } => {
                // Handle sync response from peer
            }
        }
    }
}
