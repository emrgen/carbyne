use crate::{Client, ClientId, Id};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone)]
struct EncoderMap<T: Clone + Hash + Eq> {
    items: Vec<T>,
    ids: HashMap<T, usize>,
}

impl<T: Clone + Hash + Eq> EncoderMap<T> {
    fn new() -> Self {
        EncoderMap {
            items: Vec::new(),
            ids: HashMap::new(),
        }
    }

    fn encode(&mut self, item: &T) -> u32 {
        if let Some(entry) = self.ids.get(item) {
            *entry as u32
        } else {
            let index = self.items.len();
            self.items.push(item.clone());
            self.ids.insert(item.clone(), index);
            index as u32
        }
    }

    fn encoded(&self, item: &T) -> Option<u32> {
        self.ids.get(item).map(|&index| index as u32)
    }

    fn decoded(&self, index: u32) -> Option<&T> {
        self.items.get(index as usize)
    }
}

#[derive(Clone)]
struct ClientMap {
    inner: EncoderMap<Client>,
}

impl ClientMap {
    fn get_client(&self, client_id: ClientId) -> Option<&Client> {
        self.inner.decoded(client_id)
    }
    fn get_client_id(&self, client: &Client) -> Option<ClientId> {
        self.inner.encoded(client).map(|index| index as ClientId)
    }

    fn encode(&mut self, client: &Client) -> u32 {
        self.inner.encode(client)
    }
}

struct BiMap<L: Clone + Hash + Eq, R: Clone + Hash + Eq> {
    items: Vec<(L, R)>,
    left: HashMap<L, usize>,
    right: HashMap<R, usize>,
}

impl<L: Clone + Hash + Eq, R: Clone + Hash + Eq> BiMap<L, R> {
    fn new() -> Self {
        BiMap {
            items: Vec::new(),
            left: HashMap::new(),
            right: HashMap::new(),
        }
    }

    fn get_left(&self, right: &R) -> Option<&L> {
        self.right.get(right).map(|&index| &self.items[index].0)
    }

    fn get_right(&self, left: &L) -> Option<&R> {
        self.left.get(left).map(|&index| &self.items[index].1)
    }

    fn insert(&mut self, left: L, right: R) {
        self.left.insert(left.clone(), self.items.len());
        self.right.insert(right.clone(), self.items.len());
        self.items.push((left, right))
    }
}
