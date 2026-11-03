use crate::sync::message::DbMessage;
use tokio::sync::mpsc::{Receiver, Sender};

pub type PeerId = String;
pub type PeerTx = Sender<DbMessage>;
pub type PeerRx = Receiver<DbMessage>;

pub struct PeerConnection {
    pub id: PeerId,
    pub tx: PeerTx,
}
