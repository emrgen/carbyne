
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Sub};

use serde::{Serialize, Serializer};
use uuid::Uuid;

use crate::bimapid::{ClientId, ClientMap, ClientMapper};
use crate::change::ChangeId;
use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::hash::calculate_hash;
use crate::Type;

/// 32 bits Lamport Clock tick
pub type ClockTick = u32;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Client {
    #[cfg(feature = "uuid-client")]
    UUID(Uuid),
    #[cfg(feature = "string-client")]
    String(String),
    #[cfg(feature = "u64-client")]
    U64(u64),
}

impl Default for Client {
    fn default() -> Self {
        #[cfg(feature = "uuid-client")]
        return Client::UUID(Uuid::new_v4());
        #[cfg(feature = "string-client")]
        return Client::String("".to_string());
        #[cfg(feature = "u64-client")]
        return Client::U64(0);
    }
}

impl Client {
    pub fn from_uuid(uuid: Uuid) -> Client {
        #[cfg(feature = "uuid-client")]
        return Client::UUID(uuid);
        #[cfg(not(feature = "uuid-client"))]
        panic!("UUID client is not implemented");
    }

    pub fn from_string(string: String) -> Client {
        #[cfg(feature = "string-client")]
        return Client::String(string);
        #[cfg(not(feature = "string-client"))]
        panic!("String client is not implemented");
    }

    pub(crate) fn from_u64(u64: u64) -> Client {
        #[cfg(feature = "u64-client")]
        return Client::U64(u64);
        #[cfg(not(feature = "u64-client"))]
        panic!("U64 client is not implemented");
    }

    pub fn from_bytes(bytes: &[u8]) -> Client {
        #[cfg(feature = "uuid-client")]
        if bytes.len() == 16 {
            let mut array = [0; 16];
            array.copy_from_slice(bytes);
            return Client::UUID(Uuid::from_bytes(array));
        }

        #[cfg(feature = "string-client")]
        if let Ok(string) = String::from_utf8(bytes.to_vec()) {
            return Client::String(string);
        }

        #[cfg(feature = "u64-client")]
        if bytes.len() == 8 {
            let mut array = [0; 8];
            array.copy_from_slice(bytes);
            return Client::U64(u64::from_be_bytes(array));
        }

        panic!("Invalid bytes for Client");
    }

    pub(crate) fn as_uuid(&self) -> Uuid {
        #[cfg(feature = "uuid-client")]
        if let Client::UUID(uuid) = self {
            return *uuid;
        }

        panic!("Client is not a UUID");
    }

    pub(crate) fn as_string(&self) -> String {
        #[cfg(feature = "string-client")]
        if let Client::String(string) = self {
            return string.clone();
        }

        panic!("Client is not a String");
    }

    pub(crate) fn as_u64(&self) -> u64 {
        #[cfg(feature = "u64-client")]
        if let Client::U64(u64) = self {
            return *u64;
        }

        panic!("Client is not a U64");
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            #[cfg(feature = "uuid-client")]
            Client::UUID(uuid) => uuid.as_bytes().to_vec(),
            #[cfg(feature = "string-client")]
            Client::String(string) => string.as_bytes().to_vec(),
            #[cfg(feature = "u64-client")]
            Client::U64(u64) => u64.to_be_bytes().to_vec(),
        }
    }
}

impl Serialize for Client {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{}", self).serialize(serializer)
    }
}

impl Encode for Client {
    fn encode<T: Encoder>(&self, e: &mut T, ctx: &mut EncodeContext) {
        match self {
            #[cfg(feature = "uuid-client")]
            Client::UUID(client) => e.uuid(client.as_bytes().as_slice()),
            #[cfg(feature = "string-client")]
            Client::String(client) => e.string(client),
            #[cfg(feature = "u64-client")]
            Client::U64(client) => e.u64(client),
        }
    }
}

impl Decode for Client {
    fn decode<T: Decoder>(d: &mut T, ctx: &DecodeContext) -> Result<Self, String>
    where
        Self: Sized,
    {
        #[cfg(feature = "uuid-client")]
        let uuid = d.uuid()?;
        let uuid = Uuid::from_slice(&uuid).expect("Invalid UUID");
        return Ok(Client::UUID(uuid));

        #[cfg(feature = "string-client")]
        return Ok(Client::String(d.string()?));

        #[cfg(feature = "u64-client")]
        return Ok(Client::U64(d.u64()?));

        Err("Invalid version for Client".to_string())
    }
}

impl From<String> for Client {
    fn from(value: String) -> Self {
        Client::from_string(value)
    }
}

impl From<Uuid> for Client {
    fn from(value: Uuid) -> Self {
        Client::from_uuid(value)
    }
}

impl From<u64> for Client {
    fn from(value: u64) -> Self {
        Client::from_u64(value)
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "uuid-client")]
            Client::UUID(uuid) => write!(f, "{}", uuid),
            #[cfg(feature = "string-client")]
            Client::String(string) => write!(f, "{}", string),
            #[cfg(feature = "u64-client")]
            Client::U64(u64) => write!(f, "{}", u64),
        }
    }
}

pub(crate) trait Split {
    type Target;
    fn split(&self, offset: u32) -> Result<(Self::Target, Self::Target), String>;
}

#[derive(Debug, Clone, Copy, Default, Hash)]
pub struct Id {
    pub(crate) client: ClientId,
    pub(crate) clock: ClockTick,
}

impl Id {
    pub(crate) fn adjust(&self, before: &ClientMap, after: &ClientMap) -> Id {
        let client = before.get_client(&self.client).unwrap();
        let new_client = after.get_client_id(client).unwrap();

        Id::new(*new_client, self.clock)
    }
}

impl Id {
    #[inline]
    pub fn new(client: ClientId, clock: ClockTick) -> Id {
        Id { client, clock }
    }

    #[inline]
    pub(crate) fn eq_opt(a: &Option<Id>, b: &Option<Id>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => {
                a.client == b.client && a.compare_without_client(&b) == Ordering::Equal
            }
            (None, None) => true,
            _ => false,
        }
    }

    #[inline]
    pub(crate) fn compare_without_client(&self, other: &Id) -> Ordering {
        if self.client != other.client {
            return self.client.cmp(&other.client);
        }

        self.clock.cmp(&other.clock)
    }

    // pub(crate) fn compare_with_client(&self, other: &Id) -> Ordering {
    //     if self.client != other.client {
    //         let left = calculate_hash(&format!("{}{}", self.client, self.clock));
    //         let right = calculate_hash(&format!("{}{}", self.client, other.clock));
    //         return left.cmp(&right);
    //     }
    //
    //     self.compare_without_client(other)
    // }

    pub(crate) fn compare(&self, other: &Id, clients: &ClientMap) -> Ordering {
        if self.client != other.client {
            if clients.size() == 0 {
                panic!("Cannot compare Ids from different clients without a client map")
            }

            let client = clients.get_client(&self.client).unwrap();
            let other_client = clients.get_client(&other.client).unwrap();
            let left = calculate_hash(&format!("{}{}", client, self.clock));
            let right = calculate_hash(&format!("{}{}", other_client, other.clock));
            return left.cmp(&right);
        }
        self.compare_without_client(other)
    }

    #[inline]
    pub(crate) fn next(&self) -> Id {
        Id::new(self.client, self.clock + 1)
    }

    #[inline]
    pub(crate) fn range(&self, size: u32) -> IdRange {
        IdRange::new(self.client, self.clock, self.clock + size - 1)
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        format!("Id({}, {})", self.client, self.clock).serialize(serializer)
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.client, self.clock)
    }
}

impl WithId for Id {
    #[inline]
    fn id(&self) -> Id {
        *self
    }
}

impl From<(ClientId, ClockTick)> for Id {
    fn from((client, clock): (ClientId, ClockTick)) -> Self {
        Id::new(client, clock)
    }
}

impl From<IdRange> for Id {
    fn from(value: IdRange) -> Self {
        Id::new(value.client, value.start)
    }
}

impl Sub<ClockTick> for Id {
    type Output = Id;

    fn sub(self, rhs: ClockTick) -> Self::Output {
        Id::new(self.client, self.clock - rhs)
    }
}

impl Add<ClockTick> for Id {
    type Output = Id;

    fn add(self, rhs: ClockTick) -> Self::Output {
        Id::new(self.client, self.clock + rhs)
    }
}

impl Add<ClockTick> for &Id {
    type Output = Id;

    fn add(self, rhs: ClockTick) -> Self::Output {
        Id::new(self.client, self.clock + rhs)
    }
}

impl PartialEq<Self> for Id {
    fn eq(&self, other: &Self) -> bool {
        self.client == other.client && self.clock == other.clock
    }
}

impl Eq for Id {}

impl PartialOrd<Self> for Id {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Id {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare_without_client(other)
    }
}

impl Encode for Id {
    fn encode<T: Encoder>(&self, e: &mut T, _ctx: &mut EncodeContext) {
        e.u32(self.client);
        e.u32(self.clock);
    }
}

impl Decode for Id {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<Self, String> {
        let client = d.u32()?;
        let clock = d.u32()?;

        Ok(Id::new(client, clock))
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct IdRange {
    pub(crate) client: ClientId,
    pub(crate) start: ClockTick,
    pub(crate) end: ClockTick,
}

impl IdRange {
    pub(crate) fn new(client: ClientId, start: ClockTick, end: ClockTick) -> IdRange {
        IdRange { client, start, end }
    }

    #[inline]
    pub(crate) fn size(&self) -> ClockTick {
        self.end - self.start + 1
    }

    #[inline]
    pub(crate) fn eq_opt(a: Option<&IdRange>, b: Option<&IdRange>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => {
                a.client == b.client && a.compare_without_client(b) == std::cmp::Ordering::Equal
            }
            (None, None) => true,
            _ => false,
        }
    }

    #[inline]
    pub(crate) fn equals(&self, other: &IdRange) -> bool {
        self.client == other.client && self.start == other.start && self.end == other.end
    }

    #[inline]
    pub(crate) fn start_id(&self) -> Id {
        Id::new(self.client, self.start)
    }

    #[inline]
    pub(crate) fn end_id(&self) -> Id {
        Id::new(self.client, self.end)
    }

    // Compare two Ids, considering the client field if they are different
    pub(crate) fn compare(&self, other: &IdRange, clients: &ClientMap) -> std::cmp::Ordering {
        if self.client != other.client {
            let client = clients.get_client(&self.client).unwrap();
            let other_client = clients.get_client(&other.client).unwrap();
            return calculate_hash(&format!("({}, {})", client, self.start)).cmp(&calculate_hash(
                &format!("({}, {})", other_client, other.start),
            ));
        }

        self.compare_without_client(other)
    }

    // Compare two Ids assuming the clients are same
    // e.g. [1...3] < [2..2] < [1...3] will help to find [1...3] using [2..2]
    #[inline]
    pub fn compare_without_client(&self, other: &IdRange) -> std::cmp::Ordering {
        assert_eq!(self.client, other.client);

        if self.end < other.start {
            std::cmp::Ordering::Less
        } else if other.end < self.start {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }

    // split the IdRange at the given offset, left side will have the offset size
    pub(crate) fn split(&self, offset: u32) -> Result<(IdRange, IdRange), String> {
        if offset == 0 || offset >= self.size() {
            return Err("Cannot split IdRange at invalid position".to_string());
        }

        Ok((
            IdRange::new(self.client, self.start, self.start + offset - 1),
            IdRange::new(self.client, self.start + offset, self.end),
        ))
    }

    #[inline]
    pub(crate) fn adjust(&self, before: &ClientMap, after: &ClientMap) -> IdRange {
        self.id().adjust(before, after).range(self.size())
    }
}

impl From<ChangeId> for IdRange {
    fn from(change: ChangeId) -> Self {
        IdRange {
            client: change.client,
            start: change.start,
            end: change.end,
        }
    }
}

impl Display for IdRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.client, self.start, self.end)
    }
}

impl Serialize for IdRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        format!("Range({}, {}, {})", self.client, self.start, self.end).serialize(serializer)
    }
}

impl Encode for IdRange {
    fn encode<T: Encoder>(&self, e: &mut T, _cx: &mut EncodeContext) {
        e.u32(self.client);
        e.u32(self.start);
        e.u32(self.size());
    }
}

impl Decode for IdRange {
    fn decode<T: Decoder>(d: &mut T, _cx: &DecodeContext) -> Result<Self, String> {
        let client = d.u32()?;
        let start = d.u32()?;
        let size = d.u32()?;

        Ok(IdRange::new(client, start, start + size - 1))
    }
}

impl WithId for IdRange {
    #[inline]
    fn id(&self) -> Id {
        self.clone().into()
    }
}

impl Add<IdRange> for IdRange {
    type Output = IdRange;

    fn add(self, other: IdRange) -> IdRange {
        if (self.client != other.client) || (self.end + 1 != other.start) {
            panic!("Cannot add non-adjacent Ids")
        }

        IdRange::new(
            self.client,
            self.start.min(other.start),
            self.end.max(other.end),
        )
    }
}

/// WithId trait is used to get the ID of an object
pub(crate) trait WithId {
    fn id(&self) -> Id;
}

pub(crate) trait IdComp {
    // check if self is less, equal or greater than other
    fn comp_id(&self, other: &Id) -> Ordering;
}

pub(crate) trait WithTarget {
    fn set_target(&self, target: Type);
    fn get_target(&self) -> Option<Type>;
}

/// WithIdRange trait is used to get the ID range of an object
pub(crate) trait WithIdRange {
    fn range(&self) -> IdRange;
}

impl Split for IdRange {
    type Target = IdRange;
    #[inline]
    fn split(&self, at: ClockTick) -> Result<(IdRange, IdRange), String> {
        self.split(at)
    }
}

impl std::fmt::Debug for IdRange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Id({:?}, {:?}, {:?})", self.client, self.start, self.end)
    }
}

impl From<Id> for IdRange {
    fn from(value: Id) -> Self {
        IdRange::new(value.client, value.clock, value.clock)
    }
}

impl PartialEq<Self> for IdRange {
    fn eq(&self, other: &Self) -> bool {
        self.compare_without_client(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for IdRange {}

impl PartialOrd<Self> for IdRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(std::cmp::Ord::cmp(self, other))
    }
}

impl Ord for IdRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare_without_client(other)
    }
}

impl std::hash::Hash for IdRange {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.client.hash(state);
        self.start.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::bimapid::ClientMap;

    use super::*;

    #[test]
    fn test_compare() {
        let mut clients = ClientMap::new();
        clients.get_or_insert(&Client::default());
        clients.get_or_insert(&Client::default());

        let id1 = IdRange::new(0, 1, 1);
        let id2 = IdRange::new(0, 1, 1);
        let id3 = IdRange::new(0, 2, 1);
        let id4 = IdRange::new(0, 2, 2);

        // compare without client
        assert_eq!(id1.compare_without_client(&id2), Ordering::Equal);
        assert_eq!(id1.compare_without_client(&id3), Ordering::Less);
        assert_eq!(id3.compare_without_client(&id1), Ordering::Greater);
        assert_eq!(id3.compare_without_client(&id4), Ordering::Less);
    }

    #[test]
    fn test_client() {
        let client = Client::default();
        println!("{}", client);
    }
}
