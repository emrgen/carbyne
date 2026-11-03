use crate::index::ItemIndexMap;
use crate::item::WithIndex;
use crate::Type;
use fractional_index::FractionalIndex;
use skiplist::skipmap::SkipMap;

#[derive(Debug, Default)]
pub(crate) struct SkipIndexMap {
    map: SkipMap<FractionalIndex, Type>,
}

impl Clone for SkipIndexMap {
    fn clone(&self) -> Self {
        let mut map = SkipMap::new();
        for (k, v) in self.map.iter() {
            map.insert(k.clone(), v.clone());
        }
        SkipIndexMap { map }
    }
}

impl ItemIndexMap<Type> for SkipIndexMap {
    fn size(&self) -> u32 {
        self.map.len() as u32
    }

    fn at_index(&self, index: u32) -> Option<&Type> {
        self.map.iter().nth(index as usize).map(|(_, v)| v)
    }

    fn index_of(&self, item: &Type) -> i32 {
        self.map
            .iter()
            .position(|(_, v)| v == item)
            .map_or(-1, |pos| pos as i32)
    }

    fn insert(&mut self, item: Type) {
        self.map.insert(item.index(), item);
    }

    fn remove(&mut self, item: &Type) {
        self.map.remove(&item.index());
    }

    fn contains(&self, item: &Type) -> bool {
        self.map.contains_key(&item.index())
    }
}
