use hashbrown::HashMap;

use crate::bimapid::ClientId;
use crate::ClockTick;

pub(crate) type VersionId = u64;

pub(crate) type ClientTick = HashMap<VersionId, ClockTick>;

// Version is a map of client ids to a map of version ids to clocks.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct Version {
    pub(crate) clocks_diffs: HashMap<ClientId, ClientTick>,
    pub(crate) clocks: HashMap<ClientId, ClientTick>,
}

// impl Version {
//     pub(crate) fn new() -> Self {
//         Default::default()
//     }
//
//     pub(crate) fn insert(&mut self, client_id: ClientId, version_id: VersionId, clock: Clock) {
//         let client = self.clocks.entry(client_id).or_insert_with(HashMap::new);
//         client.insert(version_id, clock);
//     }
//
//     pub(crate) fn remove(&mut self, client_id: ClientId) {
//         self.clocks.remove(&client_id);
//     }
//
//     pub(crate) fn get(&self, client_id: ClientId) -> Option<&ClientTick> {
//         self.clocks.get(&client_id)
//     }
// }
//
// impl Serialize for Version {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         self.clocks.serialize(serializer)
//     }
// }
//
// impl Deserialize<'_> for Version {
//     fn deserialize<D>(deserializer: D) -> Result<Version, D::Error>
//     where
//         D: serde::Deserializer<'_>,
//     {
//         let clocks = HashMap::deserialize(deserializer)?;
//         Ok(Version {
//             clocks_diffs: Default::default(),
//             clocks,
//         })
//     }
// }
//
// impl Encode for Version {
//     fn encode(&self, encoder: &mut crate::Encoder) {
//         let bytes = bincode::serialize(&self).unwrap();
//         encoder.write(&bytes);
//     }
// }
