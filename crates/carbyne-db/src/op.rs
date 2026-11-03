use carbyne::Change;
use serde::{Deserialize, Serialize};

/// Serializable wrapper for carbyne::Change
/// Phase 3.5: Used for wire transmission and storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedOp {
    // carbyne::Change has flag and id; for now encode as placeholder
    #[serde(skip)]
    pub change: Option<Change>,
}

impl From<Change> for SerializedOp {
    fn from(_change: Change) -> Self {
        // TODO: Serialize carbyne::Change properly
        // For Phase 3.5: placeholder
        Self { change: None }
    }
}

impl From<SerializedOp> for Change {
    fn from(_sop: SerializedOp) -> Self {
        // TODO: Deserialize from SerializedOp
        Change::default()
    }
}
