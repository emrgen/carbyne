struct EncoderMap<T: Clone + Hash> {
  items: Vec<T>,
  ids: HashMap<T, usize>,
}

impl <T> EncoderMap<T> {
  fn new() -> Self {
    BiMap {
      items: Vec::new(),
      ids: HashMap::new(),
    }
  }

  fn encode(&self, item: &T) -> u32 {
      if let Some(entry) = self.ids.get(item) {
          return entry;
      } else {
          let index = self.items.len();
          self.items.push(item.clone());
          self.ids.insert(item.clone(), index);
          return index;
      }
  }

  fn decode(&self, index: u32) -> Option<T> {
    self.items.get(index as usize)
  }
}

#[derive(Default, Clone)]
struct ClientMap {
  inner: EncoderMap<Client>,
}

impl ClientMap {
  fn get_client(&self, client_id: ClientId) -> Option<Client> {}
  fn get_client_id(&self, client: Client) -> Option<ClientId> {}
}
