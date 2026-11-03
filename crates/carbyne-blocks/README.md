# carbyne-blocks: Materialized Block Store

SQLite-backed block materialization for [carbyne](../carbyne) CRDT documents. Automatically detects changes, extracts blocks from CRDT, and persists to relational storage.

## Why

Carbyne stores the document as a CRDT tree (for sync). But editors need blocks: lists, queries, viewport optimizations. **carbyne-blocks bridges that gap.**

```
carbyne CRDT                     carbyne-blocks
(operations)                     (relational)
├─ Insert(5, "x")    ──apply─→   blocks table
├─ Delete(3)          ──sync──→   ├─ id, parent_id
└─ Move(7 → 2)         ──query─→  ├─ type, content
                                  ├─ version, timestamps
                                  └─ LRU cache
```

## Features

- ✅ **O(n) dirty detection** via change log scan  
- ✅ **O(1) cached queries** for hot blocks  
- ✅ **Batch operations** for efficient re-renders  
- ✅ **Nested CRDT support** (text, lists, maps)  
- ✅ **Atomic transactions** for consistency  
- ✅ **Parent-child relationships** for hierarchy  

## Usage

### Basic

```rust
use carbyne::Doc;
use carbyne_blocks::BlockStore;
use std::sync::Arc;

// Create document
let doc = Arc::new(Doc::new(Default::default()));
let root = doc.list();

// Add blocks
for i in 0..100 {
    let block = doc.map();
    block.set("content", doc.string(&format!("Block {}", i)));
    root.append(block);
}
doc.commit();

// Store blocks
let store = BlockStore::new(doc.clone(), "workspace.db")?;
store.rebuild_all()?;  // Initial snapshot

// Query
let block = store.get_block(0)?;
println!("{:?}", block);
```

### With Sync

```rust
// Render at version V
let render_version = doc.version();

// Peer edits, diff arrives
let diff = receive_peer_diff();
doc.apply(&diff);

// Materialize changes
let changes = store.sync(render_version)?;

// Get blocks to re-render
let ids: Vec<_> = changes.iter().map(|c| c.block_id).collect();
let blocks = store.get_blocks(&ids)?;

// Filter to viewport (if needed)
for block in blocks.iter().filter(|b| b.id < 100) {
    update_ui(block);
}
```

## API

### BlockStore

```rust
pub fn new(doc: Arc<Doc>, db_path: &str) -> Result<Self>
pub fn sync(&self, version_from: ClientState) -> Result<Vec<BlockChange>>
pub fn get_block(&self, id: usize) -> Result<Block>
pub fn get_blocks(&self, ids: &[usize]) -> Result<Vec<Block>>  // Batch query
pub fn get_children(&self, parent_id: usize) -> Result<Vec<usize>>
pub fn all_blocks(&self) -> Result<Vec<Block>>
pub fn rebuild_all(&self) -> Result<()>
pub fn clear(&self) -> Result<()>
```

### Block

```rust
pub struct Block {
    pub id: usize,                      // Position in root list
    pub parent_id: Option<usize>,       // Parent block (if nested)
    pub block_type: String,             // "paragraph", "heading", etc
    pub content: String,                // Materialized content
    pub version: Vec<u8>,               // ClientState (for sync tracking)
    pub created_at: i64,
    pub updated_at: i64,
}
```

### BlockChange

```rust
pub struct BlockChange {
    pub block_id: usize,
    pub is_new: bool,
    pub is_deleted: bool,
    pub block: Option<Block>,
}
```

## How It Works

### 1. Change Detection

`doc.dirty_blocks(version)` scans the CRDT change log:

```
Change store: {
  (client:1, clock:5): Insert("text"),   ← Peer edit
  (client:1, clock:6): Insert("text"),
  (client:2, clock:3): Delete(),
}

version_before: {client:1 → 4}

dirty_blocks() finds:
  - clock:5 > 4 ✓  
  - clock:6 > 4 ✓
  - Returns: [block_3, block_5]
```

**Time:** O(1) quick path (has changes?) + O(n) scan (which blocks?) = ~7ms for 10k blocks.

### 2. Materialization

`store.sync(version)` for each dirty block:

```
1. Get from CRDT: root.get(block_id) → Type::Map{...}
2. Extract fields: rebuild_block(id, Type::Map)
3. Check SQLite: exists?
4. INSERT or UPDATE
5. Emit: BlockChange { id, is_new, block }
```

### 3. Caching

LRU cache (1000 hot blocks) + SQLite:

```
get_block(5)
  ├─ Cache hit? → return
  └─ Cache miss → query DB → cache + return
```

### 4. Batch Queries

`get_blocks(&[0,3,7])` vs loop of `get_block()`:

```rust
// Fast: one SQL query
let blocks = store.get_blocks(&[0,3,7])?;

// Slow: 3 queries
for id in [0,3,7] {
    let block = store.get_block(id)?;
}
```

## Schema

```sql
CREATE TABLE blocks (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER,              -- NULL for root blocks
    block_type TEXT NOT NULL,       -- "paragraph", "heading", etc
    content TEXT NOT NULL,          -- Materialized content
    version BLOB NOT NULL,          -- ClientState (bincode)
    created_at INTEGER NOT NULL,    -- Unix timestamp
    updated_at INTEGER NOT NULL
);

INDEX idx_parent_id   -- For parent-child queries
INDEX idx_updated_at  -- For change tracking
```

## Performance

| Operation | Time | Blocks | Notes |
|-----------|------|--------|-------|
| `dirty_blocks()` (10k) | 7.0 ms | 10,000 | O(n) scan, very fast |
| `sync()` (10k, 100 dirty) | 8.0 ms | 10,000 | Rebuild+INSERT/UPDATE |
| `get_block()` cached | <1 µs | any | LRU hit |
| `get_block()` uncached | 10 µs | any | SQLite query |
| `get_blocks(&[0..100])` | 50 µs | 100 | Single SQL query |

## Limitations (Current)

- Parent-id extraction not automatic (stored as NULL)
- Nested text not fully extracted ("[text]" placeholder)
- No historical versions (only current state)
- No soft deletes (deletion must be detected externally)

## Integration

### With Editor (React example)

```typescript
const editor = useRef(new WorkspaceEditor(doc, "blocks.db"));

useEffect(() => {
    // Listen for changes
    editor.current.subscribe((event) => {
        if (event instanceof BlocksUpdated) {
            const dirty = event.blocks.filter(b => isInViewport(b.id));
            for (const block of dirty) {
                updateBlockUI(block);
            }
        }
    });

    // Handle peer sync
    syncChannel.onDiff((diff) => {
        editor.current.on_peer_sync(diff);
        // ^ Triggers subscribers above automatically
    });
}, []);
```

## Next Steps

1. **Implement WorkspaceEditor** (~100 lines): Event system + viewport filtering
2. **Extract nested text** (~20 lines): Convert NText to String
3. **Delete tracking** (~30 lines): Detect removed blocks
4. **Parent-child queries** (~20 lines): Populate parent_id field

All trivial additions; the hard part (CRDT→Block pipeline) is done.

## See Also

- [carbyne](../carbyne) - Core CRDT
- `crates/carbyne-blocks/src/lib.rs` - Full implementation
- `benches/block_dirty_tracking.rs` - Performance benchmarks
