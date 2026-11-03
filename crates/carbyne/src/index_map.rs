use std::ops::{AddAssign, Range};

// IndexRef holds an index that needs to be mapped to get the actual index
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct IndexRef {
    pub(crate) index: usize,
    pub(crate) mapper: u32,
}

impl IndexRef {
    pub(crate) fn new(index: usize, mapper: u32) -> IndexRef {
        IndexRef { index, mapper }
    }
}

// Maps an old index to current index
#[derive(Clone, Debug)]
pub(crate) struct IndexMapper {
    map: Vec<IndexMap>,
}

impl IndexMapper {
    fn new() -> Self {
        Self::default()
    }

    // resets the mapper to initial state
    pub(crate) fn reset(&mut self) {
        self.map.drain(1..);
    }

    pub(crate) fn push(&mut self, map: IndexMap) -> usize {
        self.map.push(map);

        self.map.len() - 1
    }

    // map the index_ref to the final index
    pub(crate) fn map_ref(&self, index_ref: &IndexRef) -> usize {
        self.map(index_ref.mapper, index_ref.index as usize)
    }

    fn map(&self, after: u32, index: usize) -> usize {
        let mut pos = index;

        self.map[(after as usize + 1)..]
            .iter()
            .fold(pos, |idx, m| m.map(idx))
    }

    fn unmap(&self, pos: usize) -> usize {
        let mut pos = pos;
        self.map.iter().rev().fold(pos, |idx, m| m.unmap(idx))
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn last_index(&self) -> usize {
        self.map.last().unwrap().last_index()
    }
}

impl Default for IndexMapper {
    fn default() -> IndexMapper {
        IndexMapper {
            map: vec![IndexMap::default()],
        }
    }
}

/// IndexMap is used to map index changes when insert
#[derive(Clone, Debug)]
pub(crate) struct IndexMap {
    index: usize,
}

const INF: u32 = u32::MAX;

impl Default for IndexMap {
    fn default() -> IndexMap {
        IndexMap { index: usize::MAX }
    }
}

impl IndexMap {
    pub(crate) fn insert(at: usize) -> IndexMap {
        IndexMap { index: at }
    }

    pub(crate) fn remove(at: usize) -> IndexMap {
        IndexMap {
            index: at | 1 << 31,
        }
    }

    #[inline]
    fn last_index(&self) -> usize {
        self.index
    }

    #[inline]
    fn is_insert(&self) -> bool {
        self.index & (1 << 31) == 0
    }

    #[inline]
    fn is_remove(&self) -> bool {
        self.index & (1 << 31) != 0
    }

    #[inline]
    fn index(&self) -> usize {
        self.index & (1 << 31 - 1)
    }

    fn map(&self, index: usize) -> usize {
        if self.is_insert() {
            if self.index <= index {
                index + 1
            } else {
                index
            }
        } else {
            if self.index <= index {
                index - 1
            } else {
                index
            }
        }
    }

    fn unmap(&self, pos: usize) -> usize {
        if self.is_insert() {
            if self.index < pos {
                pos - 1
            } else {
                pos
            }
        } else {
            if self.index < pos {
                pos + 1
            } else {
                pos
            }
        }
    }
}

impl From<IndexMap> for Range<usize> {
    fn from(map: IndexMap) -> Self {
        Range {
            start: map.index,
            end: map.last_index(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_index_mapper_1() {
        let mut mapper = IndexMapper::default();
        mapper.push(IndexMap::insert(0));
        mapper.push(IndexMap::insert(1));

        // index compression merge the two maps, so the after is 1 for both insertions
        assert_eq!(mapper.map(0, 1), 3);
        assert_eq!(mapper.map(1, 0), 0);

        // mapper.push(IndexMap::insert(1, 1));
        // assert_eq!(mapper.map(1, 0), 0);
        // assert_eq!(mapper.map(1, 1), 3);
        // assert_eq!(mapper.map(2, 1), 2);
        // assert_eq!(mapper.map(3, 1), 1);
    }

    #[test]
    fn test_index_mapper_2() {
        let mut mapper = IndexMapper::default();
        // all the indexes after 10 should be increased by 1
        mapper.push(IndexMap::insert(10));

        assert_eq!(mapper.map(0, 0), 0);
        assert_eq!(mapper.map(0, 9), 9);
        assert_eq!(mapper.map(0, 10), 11);
        assert_eq!(mapper.map(0, 13), 14);
        assert_eq!(mapper.map(1, 10), 10);
        assert_eq!(mapper.map(1, 11), 11);
    }

    #[test]
    fn test_index_mapper_ranges() {
        let mut mapper = IndexMapper::default();
        mapper.push(IndexMap::insert(0));
        mapper.push(IndexMap::insert(3));
        mapper.push(IndexMap::insert(5));
        mapper.push(IndexMap::insert(8));
    }

    #[test]
    fn test_index_mapper_ranges_2() {
        let mut mapper = IndexMapper::default();
        mapper.push(IndexMap::insert(0));
        mapper.push(IndexMap::insert(3));
        mapper.push(IndexMap::insert(5));
        mapper.push(IndexMap::insert(8));

        assert_eq!(mapper.map(0, 2), 4);
        assert_eq!(mapper.map(0, 3), 6);

        mapper.push(IndexMap::insert(2));
    }
}
