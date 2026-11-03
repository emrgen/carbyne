use crate::id::Id;

/// The ClientIdStore struct is used to store client IDs and their corresponding clocks.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub(crate) struct ClientIdStore {
    pub(crate) items: Vec<Vec<u32>>,
}

impl ClientIdStore {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub(crate) fn insert(&mut self, id: Id) {
        let Id { client, clock, .. } = &id;
        if self.items.len() <= *client as usize {
            self.items.resize(*client as usize + 1, Vec::new());
        }

        let mut ids = self.items.get_mut(*client as usize).unwrap();
        ids.push(*clock);
    }

    pub(crate) fn contains(&self, id: &Id) -> bool {
        let Id { client, clock, .. } = id;
        if let Some(ids) = self.items.get(*client as usize) {
            find_index(ids, clock).is_some()
        } else {
            false
        }
    }

    pub(crate) fn clear(&mut self) {
        self.items.iter_mut().for_each(|ids| ids.clear());
    }
}

fn find_index<T: Ord>(arr: &[T], find: &T) -> Option<usize> {
    let length = arr.len() as i32;
    if length == 0 {
        return None;
    }

    let mut half = length / 2;
    let mut hind: i32 = (length - 1);
    let mut lind = 0;
    let mut current = &arr[half as usize];

    while lind <= hind {
        match current.cmp(find) {
            std::cmp::Ordering::Equal => return Some(half as usize),
            std::cmp::Ordering::Less => lind = (half + 1),
            std::cmp::Ordering::Greater => hind = (half - 1),
        }
        half = ((hind + lind) / 2);
        current = &arr[half as usize];
    }

    return None;
}
