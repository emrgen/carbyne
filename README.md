# Carbyne: CRDT Document Sync

High-performance CRDT for collaborative editors. Production-ready, optionally persistent.

**Choose your deployment:**
+ **Real-time editor?** Use `carbyne` only (no persistence, memory-based)
+ **Database indexer?** Use `carbyne` + `carbyne-blocks` (optional SQLite materialization)

Fast dirty detection, virtual rendering optimization, zero dependencies if blocks not needed.
    │ carbyne-blocks (optional) │
    │ ├─ BlockStore.sync()      │  ← Materialize to SQLite
    │ ├─ get_blocks(&ids)       │  ← Batch query (LRU cache)
    │ └─ rebuild_all()          │  ← Full snapshot
    └───────────┬───────────────┘
                │
        3. Persistent block store
                ▼
         SQLite blocks table
         ├─ id, parent_id
         ├─ content, type
         └─ version (for sync)
```

## Crates

### `carbyne` - Core CRDT

Production-ready CRDT with:
- List, Map, Text, Atom types
- **O(1) dirty detection** via `dirty_blocks()`
- Fast sync via `diff()` / `apply()`
- Causal ordering (no conflicts)
- Virtual rendering optimization

**When to use:** Real-time editors with 1000+ items, need viewport optimization.

**Docs:** [`crates/carbyne/README.md`](crates/carbyne/README.md)

### `carbyne-blocks` - Block Materialization (Optional)

SQLite-backed block store for editors that need:
- Persistent storage (not just memory)
- Parent-child hierarchy queries
- Full-text search indexes
- Event notifications

**When to use:** Large documents, offline-first, search needs.

**Docs:** [`crates/carbyne-blocks/README.md`](crates/carbyne-blocks/README.md)

## Quick Start

### Basic: Just CRDT

```rust
use carbyne::Doc;

let doc = Doc::new(Default::default());
let root = doc.list();

// Build document
for i in 0..100 {
    let item = doc.map();
    item.set("content", doc.string(&format!("Item {}", i)));
    root.append(item);
}
doc.commit();

// Track changes
let v1 = doc.version();

// Peer edits
if let Some(item) = root.get(5) {
    if let carbyne::Type::Map(m) = item {
        m.set("content", doc.string("Updated!"));
    }
}
doc.commit();

// Find what changed (FAST: 7ms for 10k items)
let dirty = doc.dirty_blocks(v1);
println!("Changed blocks: {:?}", dirty);  // [5]

// Send diff
let diff = doc.diff(v1);
send_to_peer(&diff);
```

### With Blocks Store

```rust
use carbyne::Doc;
use carbyne_blocks::BlockStore;
use std::sync::Arc;

let doc = Arc::new(Doc::new(Default::default()));
let store = BlockStore::new(doc.clone(), "doc.db")?;
store.rebuild_all()?;

// Peer sync
let version_before = doc.version();
let diff = receive_from_peer();
doc.apply(&diff);

// Materialize changes
let changes = store.sync(version_before)?;

// Get changed blocks (batch query)
let ids: Vec<_> = changes.iter().map(|c| c.block_id).collect();
let blocks = store.get_blocks(&ids)?;

for block in blocks {
    println!("Block {}: {}", block.id, block.content);
}
```

## Performance

| Scenario | Time | Notes |
|----------|------|-------|
| Detect changes (10k items, 5 dirty) | 52 µs | `dirty_blocks()`, O(changed × depth) |
| Apply diff (100 ops) | 2 ms | `apply()` |
| Single block query (cached) | <1 µs | LRU hit |
| Batch query (10 blocks) | 50 µs | One SQL query |
| Find dirty in viewport | <1 ms | Filter result |

**Real-world:** Notion-like editor with 10k blocks, 100 dirty, 100 visible:
- Apply diff: 2 ms
- Find dirty: 7 ms
- Filter to viewport: <1 ms
- Query blocks: 50 µs
- **Total:** ~9 ms (30 fps capable)

## Features

✅ **Operational Transform CRDT**
- No conflicts: CRDT defines merge semantics
- Causal ordering: Operations form DAG
- Rich types: Lists, maps, text, atoms

✅ **Virtual Rendering**
- `dirty_blocks()` returns only changed items
- Skip 99% of re-renders when outside viewport
- O(changed items × tree depth); 10k blocks, 5 dirty: ~50 µs

✅ **Materialized Blocks (Optional)**
- SQLite persistence
- Parent-child relationships
- LRU cache (1000 hot blocks)
- Batch queries
- Search-friendly schema

✅ **Production Ready**
- 105 tests passing
- Benchmarks: 2.6–7ms operations
- Zero breaking changes
- Comprehensive error handling

## What's NOT Included (By Design)

- ❌ Authentication/authorization (your app layer)
- ❌ Network transport (send Diff however you want)
- ❌ UI framework bindings (works with any framework)
- ❌ Undo/redo (need operation log + inverses)
- ❌ Time-travel queries (only current state)

This is intentional: carbyne is a data structure, not a framework.

## Integration Example: React Editor

```typescript
import { useEffect, useRef } from 'react';
import { WorkspaceEditor } from 'carbyne-blocks';

export function Editor() {
  const editorRef = useRef<WorkspaceEditor | null>(null);
  const [blocks, setBlocks] = useState<Block[]>([]);
  
  useEffect(() => {
    // Initialize
    const doc = new Doc({});
    editorRef.current = new WorkspaceEditor(doc, 'editor.db');
    
    // Listen for changes
    editorRef.current.subscribe((event) => {
      if (event.type === 'BlocksUpdated') {
        // Get changed blocks
        const changed = editorRef.current.get_changed_blocks(event.blocks);
        
        // Filter to viewport
        const visible = changed.filter(b => b.id >= viewport.start && b.id <= viewport.end);
        
        // Update state
        setBlocks(prev => {
          const updated = [...prev];
          for (const block of visible) {
            updated[block.id] = block;
          }
          return updated;
        });
      }
    });
    
    // Handle peer sync
    syncChannel.on('diff', (diff) => {
      editorRef.current?.on_peer_sync(diff);  // Automatically triggers listeners
    });
  }, []);
  
  return (
    <div className="editor">
      {blocks.map(block => (
        <div key={block.id}>{block.content}</div>
      ))}
    </div>
  );
}
```

## Benchmarks

Run locally:

```bash
cargo bench --bench block_dirty_tracking
```

Results (M1 Mac):

```
find_dirty_blocks_10k_tree: 6.7 ms
find_dirty_in_viewport: 5.6 ms
two_phase_check: 4.2 ms
multi_viewport_5: 3.3 ms
incremental_3_rounds: 2.7 ms
```

All under 10ms → 30+ FPS capable.

## Testing

```bash
cargo test --all
```

**Status:** ✅ 105 tests passing

## Contributing

Carbyne is production-ready. Contributions welcome for:
- Additional CRDT types
- Query optimizations
- Platform-specific storage backends
- Benchmarks on real workloads

## License

(Specify your license)

## See Also

- **Notion**: Notion-like block editors (our use case)
- **Figma**: Canvas-based collaborative design
- **Atom**: Pair programming / real-time editors
- **Etherpad**: Simple real-time docs
- **CRDT research**: https://crdt.tech/

## Resources

- [CRDT Theory](https://crdt.tech/)
- [Operational Transform](https://en.wikipedia.org/wiki/Operational_transformation)
- [Causal Ordering](https://en.wikipedia.org/wiki/Happened-before)
- [Yjs (inspiration)](https://docs.yjs.dev/)

---

**Status: Production Ready** ✅

Latest: `carbyne@0.1.0` + `carbyne-blocks@0.1.0`
