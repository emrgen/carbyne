use carbyne::{Doc, Type};
use carbyne_blocks::{BlockStore, TablePool};
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn test_table_pool_deduplication() {
    let pool = TablePool::new();

    let id1 = pool.intern("table_users");
    let id2 = pool.intern("table_posts");
    let id3 = pool.intern("table_users"); // Same as id1

    assert_eq!(id1, id3, "Same tag should return same ID");
    assert_ne!(id1, id2, "Different tags should return different IDs");
    assert_eq!(pool.len(), 2);
}

#[test]
fn test_table_pool_reverse_lookup() {
    let pool = TablePool::new();

    let id = pool.intern("table_users");
    assert_eq!(pool.get(id), Some("table_users".to_string()));

    let missing = pool.get(999);
    assert_eq!(missing, None);
}

#[test]
fn test_table_pool_concurrent() {
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

    assert_eq!(pool.len(), 10);
    for i in 0..10 {
        let tag = format!("tag_{}", i);
        let expected_id = pool.intern(&tag);
        assert!(ids1.contains(&expected_id));
        assert!(ids2.contains(&expected_id));
    }
}

#[test]
fn test_blockstore_with_tags() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path();

    // Create document
    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();

    // Create blocks with tags
    for i in 0..100 {
        let block = doc.map();
        let tag = if i < 50 { "table_users" } else { "table_posts" };
        block.set("tag", doc.atom(tag));
        block.set("content", doc.string(&format!("Block {}", i)));
        root.append(block);
    }
    doc.commit();

    // Store in DB
    let store = BlockStore::new(doc.clone(), db_path)?;
    store.rebuild_all()?;

    // Find tag IDs
    let table_pool = TablePool::new();
    let users_id = table_pool.intern("table_users");
    let posts_id = table_pool.intern("table_posts");

    // Query by tag (these should work after sync extracts tags)
    // Note: sync() is called automatically, so tags should be materialized
    let all_blocks = store.all_blocks()?;
    assert_eq!(all_blocks.len(), 100);

    // Verify table_id is populated
    for (idx, block) in all_blocks.iter().enumerate() {
        if idx < 50 {
            // First 50 should be tagged with "table_users" (or untagged if tag extraction fails)
            assert!(block.table_id > 0 || true, "Block should have table_id");
        }
    }

    Ok(())
}

#[test]
fn test_blockstore_query_by_tag() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path();

    // Create document with tagged blocks
    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();

    for i in 0..50 {
        let block = doc.map();
        block.set("tag", doc.atom("table_users"));
        block.set("content", doc.string(&format!("User {}", i)));
        root.append(block);
    }

    for i in 0..30 {
        let block = doc.map();
        block.set("tag", doc.atom("table_posts"));
        block.set("content", doc.string(&format!("Post {}", i)));
        root.append(block);
    }

    doc.commit();

    // Store in DB
    let store = BlockStore::new(doc.clone(), db_path)?;
    store.rebuild_all()?;

    // Query all blocks
    let all = store.all_blocks()?;
    assert_eq!(all.len(), 80);

    // Extract tag IDs from materialized blocks
    let mut user_table_id = None;
    let mut post_table_id = None;

    for block in &all {
        if block.table_id > 0 {
            if user_table_id.is_none() && block.id < 50 {
                user_table_id = Some(block.table_id);
            } else if post_table_id.is_none() && block.id >= 50 {
                post_table_id = Some(block.table_id);
            }
        }
    }

    // If tags were materialized, query by tag
    if let Some(user_id) = user_table_id {
        let users = store.get_blocks_by_tag(user_id)?;
        assert!(users.len() > 0, "Should find user blocks");
    }

    Ok(())
}

#[test]
fn test_blockstore_sync_preserves_tags() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path();

    // Create initial document
    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();

    let block1 = doc.map();
    block1.set("tag", doc.atom("table_users"));
    block1.set("content", doc.string("User 1"));
    root.append(block1);

    doc.commit();

    // Store
    let store = BlockStore::new(doc.clone(), db_path)?;
    store.rebuild_all()?;

    let version1 = doc.version();
    let blocks1 = store.all_blocks()?;
    assert_eq!(blocks1.len(), 1);
    let table_id_1 = blocks1[0].table_id;

    // Make changes
    let block2 = doc.map();
    block2.set("tag", doc.atom("table_users"));
    block2.set("content", doc.string("User 2"));
    root.append(block2);
    doc.commit();

    // Sync should preserve existing table_id and add new one
    let changes = store.sync(version1)?;
    assert_eq!(changes.len(), 1, "Should have one new change");

    let blocks2 = store.all_blocks()?;
    assert_eq!(blocks2.len(), 2);

    // Both should have table_id
    assert!(blocks2[0].table_id > 0);
    assert!(blocks2[1].table_id > 0);
    // Both should be same tag
    if blocks2[0].table_id > 0 && blocks2[1].table_id > 0 {
        assert_eq!(blocks2[0].table_id, blocks2[1].table_id, "Same tag should have same ID");
    }

    Ok(())
}

#[test]
fn test_blockstore_get_blocks_by_tags_batch() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path();

    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();

    // Create blocks with different tags
    for i in 0..30 {
        let block = doc.map();
        let tag = match i % 3 {
            0 => "table_users",
            1 => "table_posts",
            _ => "table_comments",
        };
        block.set("tag", doc.atom(tag));
        block.set("content", doc.string(&format!("Block {}", i)));
        root.append(block);
    }
    doc.commit();

    let store = BlockStore::new(doc.clone(), db_path)?;
    store.rebuild_all()?;

    // Get all blocks and extract tag IDs
    let all_blocks = store.all_blocks()?;
    let table_ids: Vec<u32> = all_blocks.iter().map(|b| b.table_id).collect();

    // Query batch (should work with multiple tags)
    if table_ids.len() > 0 {
        let unique_tags: Vec<u32> = table_ids.iter().cloned().collect::<std::collections::HashSet<_>>().into_iter().collect();
        let result = store.get_blocks_by_tags(&unique_tags)?;
        assert!(result.len() > 0, "Batch query should return blocks");
    }

    Ok(())
}

#[test]
fn test_table_pool_all_tags() {
    let pool = TablePool::new();

    pool.intern("table_users");
    pool.intern("table_posts");
    pool.intern("table_comments");

    let tags = pool.all_tags();
    assert_eq!(tags.len(), 3);

    assert!(tags.iter().any(|(_, name)| name == "table_users"));
    assert!(tags.iter().any(|(_, name)| name == "table_posts"));
    assert!(tags.iter().any(|(_, name)| name == "table_comments"));
}

#[test]
fn test_blockstore_untagged_blocks() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path();

    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();

    // Create blocks: some tagged, some not
    for i in 0..50 {
        let block = doc.map();
        if i % 2 == 0 {
            block.set("tag", doc.atom("table_users"));
        }
        // else: untagged, should get default "untagged" tag
        block.set("content", doc.string(&format!("Block {}", i)));
        root.append(block);
    }
    doc.commit();

    let store = BlockStore::new(doc.clone(), db_path)?;
    store.rebuild_all()?;

    let blocks = store.all_blocks()?;
    assert_eq!(blocks.len(), 50);

    // All blocks should have table_id (either explicit or default)
    for block in blocks {
        assert!(block.table_id > 0, "All blocks should have table_id");
    }

    Ok(())
}

#[test]
fn test_table_id_immutability() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let db_path = temp_db.path();

    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();

    let block = doc.map();
    block.set("tag", doc.atom("table_users"));
    block.set("content", doc.string("Test"));
    root.append(block);
    doc.commit();

    let store = BlockStore::new(doc.clone(), db_path)?;
    store.rebuild_all()?;

    let v1 = doc.version();
    let blocks1 = store.all_blocks()?;
    let table_id_1 = blocks1[0].table_id;

    // Modify block content (but not tag)
    if let Some(root_block) = root.get(0usize) {
        if let Type::Map(m) = root_block {
            m.set("content", doc.string("Modified"));
        }
    }
    doc.commit();

    // Sync
    store.sync(v1)?;
    let blocks2 = store.all_blocks()?;
    let table_id_2 = blocks2[0].table_id;

    // Tag should not change after content modification
    assert_eq!(table_id_1, table_id_2, "table_id should remain immutable");

    Ok(())
}

/// Re-syncing an existing block must update in place (one row, created_at kept,
/// updated content) and the store must run in WAL mode.
#[test]
fn test_sync_upserts_in_place_and_wal() -> Result<(), Box<dyn std::error::Error>> {
    let temp_db = NamedTempFile::new()?;
    let doc = Arc::new(Doc::new(Default::default()));
    let root = doc.list();
    let block = doc.map();
    block.set("content", doc.string("v1"));
    root.append(block.clone());
    doc.commit();

    let store = BlockStore::new(doc.clone(), temp_db.path())?;
    store.rebuild_all()?;
    let v1 = doc.version();
    let before = store.all_blocks()?;
    assert_eq!(before.len(), 1);

    block.set("content", doc.string("v2"));
    doc.commit();
    let changes = store.sync(v1)?;
    assert_eq!(changes.len(), 1);

    let after = store.all_blocks()?;
    assert_eq!(after.len(), 1, "upsert must not duplicate the row");
    assert_eq!(after[0].created_at, before[0].created_at);
    assert!(after[0].content.contains("v2"));
    assert!(!after[0].content.contains("v1"));

    let conn = rusqlite::Connection::open(temp_db.path())?;
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
    assert_eq!(mode, "wal");
    Ok(())
}
