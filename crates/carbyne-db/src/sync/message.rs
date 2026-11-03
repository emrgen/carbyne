use crate::instance::DbId;
use serde::{Deserialize, Serialize};

/// Wire message between peers.
/// Phase 3.5: Ops serialized as bytes for transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    pub db_id: DbId,
    pub lamport: u64,
    pub message_type: DbMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbMessageType {
    /// "send me ops after version `since`" (ClientState bytes)
    SyncRequest { since: Vec<u8> },
    /// Ops (serialized) covering the requested range
    SyncResponse { ops: Vec<u8> },
    /// Broadcast op from peer (gossip)
    OpsBroadcast { ops: Vec<u8> },
}
