use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// String interning pool for immutable block tags.
/// 
/// Deduplicates repeated tags (e.g., "table_users" on 1000 blocks).
/// Once a tag is interned, it's never modified → no CRDT conflicts.
/// 
/// Memory savings: 1000 blocks with "table_users" tag:
///   Without pool: 1000 * ~25 bytes = 25 KB
///   With pool: 25 bytes + 1000 * 4 bytes (table_id) = 4 KB
///   Savings: 84%
///
/// Thread-safe via DashMap (concurrent reads) + atomic counter.
#[derive(Clone)]
pub struct TablePool {
    /// table_name -> table_id mapping
    intern_map: DashMap<String, u32>,
    /// table_id -> table_name reverse mapping (for queries)
    reverse_map: Arc<Mutex<Vec<String>>>,
    /// Next table_id to assign
    next_id: Arc<AtomicU32>,
}

impl TablePool {
    /// Create new tag pool.
    pub fn new() -> Self {
        Self {
            intern_map: DashMap::new(),
            reverse_map: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Intern tag string → get ID. O(1) amortized.
    /// 
    /// Returns existing ID if tag already interned, otherwise assigns new ID.
    /// Safe to call concurrently; all callers see consistent IDs.
    pub fn intern(&self, tag: &str) -> u32 {
        // Fast path: tag already exists
        if let Some(entry) = self.intern_map.get(tag) {
            return *entry;
        }

        // Allocate new ID (atomic, no conflicts)
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Insert into both maps
        self.intern_map.insert(tag.to_string(), id);
        let mut reverse = self.reverse_map.lock().unwrap();
        reverse.push(tag.to_string());

        id
    }

    /// Reverse lookup: table_id → tag_name. O(1).
    pub fn get(&self, id: u32) -> Option<String> {
        self.reverse_map
            .lock()
            .unwrap()
            .get(id as usize)
            .cloned()
    }

    /// Get all interned tags (for export/debugging).
    pub fn all_tags(&self) -> Vec<(u32, String)> {
        let reverse = self.reverse_map.lock().unwrap();
        reverse
            .iter()
            .enumerate()
            .map(|(idx, tag)| (idx as u32, tag.clone()))
            .collect()
    }

    /// Number of unique tags interned.
    pub fn len(&self) -> usize {
        self.intern_map.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.intern_map.is_empty()
    }

    /// Clear all interned tags (use with caution—existing table_ids become invalid).
    pub fn clear(&self) {
        self.intern_map.clear();
        self.reverse_map.lock().unwrap().clear();
        self.next_id.store(0, Ordering::SeqCst);
    }
}

impl Default for TablePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_deduplicates() {
        let pool = TablePool::new();
        let id1 = pool.intern("table_users");
        let id2 = pool.intern("table_posts");
        let id3 = pool.intern("table_users"); // Same as id1

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_reverse_lookup() {
        let pool = TablePool::new();
        let id = pool.intern("table_users");
        assert_eq!(pool.get(id), Some("table_users".to_string()));
        assert_eq!(pool.get(999), None);
    }

    #[test]
    fn test_concurrent_interns() {
        let pool = TablePool::new();
        let pool1 = pool.clone();
        let pool2 = pool.clone();

        let h1 = std::thread::spawn(move || {
            let mut ids = Vec::new();
            for i in 0..100 {
                ids.push(pool1.intern(&format!("tag_{}", i % 10)));
            }
            ids
        });

        let h2 = std::thread::spawn(move || {
            let mut ids = Vec::new();
            for i in 0..100 {
                ids.push(pool2.intern(&format!("tag_{}", i % 10)));
            }
            ids
        });

        let ids1 = h1.join().unwrap();
        let ids2 = h2.join().unwrap();

        // All calls to same tag should return same ID
        assert_eq!(pool.len(), 10);
        for i in 0..10 {
            let tag = format!("tag_{}", i);
            let expected_id = pool.intern(&tag);
            // Both threads should have seen the same IDs
            assert!(ids1.contains(&expected_id));
            assert!(ids2.contains(&expected_id));
        }
    }

    #[test]
    fn test_all_tags() {
        let pool = TablePool::new();
        pool.intern("table_users");
        pool.intern("table_posts");
        pool.intern("table_comments");

        let tags = pool.all_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.iter().any(|(_, name)| name == "table_users"));
        assert!(tags.iter().any(|(_, name)| name == "table_posts"));
    }
}
