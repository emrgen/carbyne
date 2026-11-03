use crate::instance::DbInstance;
use crate::sync::message::{DbMessage, DbMessageType};
use crate::sync::peer::PeerTx;
use carbyne::{ClientState, Diff};
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
            version: self.instance.version().to_bytes(),
            message_type,
        }
    }

    /// Handle one message. Err on malformed payload or storage failure;
    /// `run` drops it so a bad peer cannot stop the loop.
    pub async fn handle_message(&self, inbound: Inbound) -> Result<(), String> {
        // db_id is per-node (create_from assigns a fresh one); doc identity lives in the diff.
        let Inbound { from, msg } = inbound;
        match msg.message_type {
            DbMessageType::SyncRequest { since } => {
                let since = ClientState::from_bytes(&since)?;
                let diff = self.instance.diff_since(since).to_bytes();
                if let Some(peer) = self.peers.get(&from) {
                    let _ = peer
                        .send(self.envelope(DbMessageType::SyncResponse { diff }))
                        .await;
                }
            }
            DbMessageType::SyncResponse { diff } | DbMessageType::OpsBroadcast { diff } => {
                let parsed = Diff::from_bytes(&diff)?;
                self.instance.apply(&parsed).map_err(|e| e.to_string())?;
                // fan-out to every peer except the sender
                let out = self.envelope(DbMessageType::OpsBroadcast { diff });
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
