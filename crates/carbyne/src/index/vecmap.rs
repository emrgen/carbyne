use crate::index::ItemIndexMap;
use fractional_index::FractionalIndex;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    index: FractionalIndex,
    value: u32,
}

struct VecMap {
    items: Vec<Item>,
}

impl ItemIndexMap<Item> for VecMap {
    fn size(&self) -> u32 {
        self.items.len() as u32
    }

    fn at_index(&self, index: u32) -> Option<&Item> {
        self.items.get(index as usize)
    }

    fn index_of(&self, item: &Item) -> i32 {
        if self.items.is_empty() {
            return 0;
        }

        // do a binary search
        let mut low = 0;
        let mut high = self.items.len() as i32 - 1;

        while low <= high {
            let mid = (low + high) / 2;
            let mid_item = &self.items[mid as usize];
            if mid_item.index == item.index {
                return mid;
            } else if mid_item.index < item.index {
                low = mid + 1;
            } else {
                high = mid - 1;
            }
        }

        low
    }

    fn insert(&mut self, item: Item) {
        let index = self.index_of(&item);
        self.items.insert(index as usize, item);
    }

    fn remove(&mut self, item: &Item) {
        let index = self.index_of(item);
        self.items.remove(index as usize);
    }

    fn contains(&self, item: &Item) -> bool {
        self.items
            .binary_search_by(|probe| probe.index.cmp(&item.index))
            .is_ok()
    }
}

#[cfg(test)]
mod test {
    use crate::index::vecmap::Item;
    use crate::index::ItemIndexMap;
    use btree_slab::BTreeMap;

    #[test]
    fn test_index_lookup_vec_btree() {
        use crate::index::vecmap::VecMap;
        use crate::index::ItemIndexMap;
        use fractional_index::FractionalIndex;

        let mut vecmap = VecMap { items: vec![] };
        let mut item = Item {
            index: FractionalIndex::default(),
            value: 0,
        };

        // for i in 0..100000 {
        //     vecmap.insert(item.clone());
        //
        //     item = Item {
        //         index: FractionalIndex::new_after(&item.index),
        //         value: i + 1,
        //     }
        // }

        // time
        // let now = std::time::Instant::now();
        //
        // for i in 0..vecmap.size() {
        //     let index = rand::random::<u32>() % vecmap.size();
        //     let item = vecmap.at_index(index);
        //     assert_eq!(item.unwrap().value, index);
        // }
        //
        // println!("Time taken: {:?}", now.elapsed());

        // let mut tree = BTreeMap::new();
        // let mut item = Item {
        //     index: FractionalIndex::default(),
        //     value: 0,
        // };
        //
        // for i in 0..4000 {
        //     tree.insert(item.index.clone(), item.clone());
        //
        //     item = Item {
        //         index: FractionalIndex::new_after(&item.index),
        //         value: i + 1,
        //     }
        // }
        //
        // // time
        // let now = std::time::Instant::now();
        //
        // for i in 0..tree.len() {
        //     let index = rand::random::<u32>() % tree.len() as u32;
        //     let item = tree.iter().nth(index as usize).map(|(_, v)| v);
        //     let item = item.unwrap();
        //     assert_eq!(item.value, index);
        // }
        //
        // println!("Time taken: {:?}", now.elapsed());
    }
}
