use crate::instance::DbId;
use serde::{Deserialize, Serialize};

/// Wire message between peers. `version` and diffs are carbyne v1 codec bytes
/// (`ClientState::to_bytes`, `Diff::to_bytes`); carbyne types have no serde Deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMessage {
    pub db_id: DbId,
    /// sender's vector clock
    pub version: Vec<u8>,
    pub message_type: DbMessageType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DbMessageType {
    /// "send me everything after `since`" (ClientState bytes)
    SyncRequest { since: Vec<u8> },
    /// Diff bytes covering the requested range
    SyncResponse { diff: Vec<u8> },
    /// one committed Diff, fan-out
    OpsBroadcast { diff: Vec<u8> },
}
