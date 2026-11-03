use crate::instance::{DbId, Op};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    pub db_id: DbId,
    pub lamport: u64,
    pub message_type: DbMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbMessageType {
    SyncRequest {
        peer_lamport: u64,
    },
    SyncResponse {
        ops: Vec<Op>,
        current_lamport: u64,
    },
    OpsBroadcast {
        ops: Vec<Op>,
        lamport: u64,
    },
}
