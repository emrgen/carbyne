use crate::bimapid::ClientMap;
use crate::change::{ChangeId, ChangeStore};
use crate::id::WithId;
use crate::{ClientFrontier, ClientState, Id};
use std::collections::{HashMap, HashSet};

/// The Frontier struct represents the most recent operations in a document from all clients.
#[derive(Default, Clone, Debug)]
pub struct Frontier {
    id: Id,
}

impl Frontier {
    /// Creates a new Frontier with the given ID.
    pub fn new(id: Id) -> Self {
        Frontier { id }
    }

    /// Returns the ID of the Frontier.
    pub fn id(&self) -> Id {
        self.id
    }
}
