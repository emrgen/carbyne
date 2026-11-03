# carbyne: Operational Transform CRDT

High-performance CRDT for collaborative document editing. Combines list/map/text/atom types with change-based sync and dirty detection for efficient virtual rendering.

## Features

- ✅ **CRDT Types**: List, Map, Text, Atom
- ✅ **O(1) dirty detection**: `dirty_blocks()` finds changed items
- ✅ **Fast sync**: Diff-based, not state-based
- ✅ **Rich nested structures**: Lists of maps with text fields
- ✅ **Causal ordering**: Operations form a DAG, not a log
- ✅ **Concurrent edits**: Merge without conflicts
- ✅ **Virtual rendering**: Know which items changed without rebuilding

## Quick Start

```rust
use carbyne::Doc;

// Create document
let doc = Doc::new(Default::default());

// Build structure
let root = doc.list();
for i in 0..100 {
    let item = doc.map();
    item.set("id", doc.atom(format!("item-{}", i)));
    item.set("text", doc.string(&format!("Item {}", i)));
    root.append(item);
}
doc.commit();

// Check what changed
let version_before = doc.version();

// Peer makes changes
if let Some(item) = root.get(5) {
    if let carbyne::Type::Map(m) = item {
        m.set("text", doc.string("Updated item 5"));
    }
}
doc.commit();

// Find dirty items
let dirty = doc.dirty_blocks(version_before);
println!("Changed: {:?}", dirty);  // [5]

// Send diff to peers
let diff = doc.diff(version_before);
send_to_peer(&diff);
```

## API

### Doc

```rust
pub fn new(opts: DocMeta) -> Self
pub fn list(&self) -> NList                                  // Root is always a list
pub fn map(&self) -> NMap
pub fn text(&self) -> NText
pub fn string(&self, s: &str) -> Type
pub fn atom(&self, v: impl Display) -> Type

pub fn version(&self) -> ClientState                         // Current state
pub fn has_changes_since(&self, state: ClientState) -> bool  // O(1) quick check
pub fn dirty_blocks(&self, state: ClientState) -> Vec<usize> // Which items changed
pub fn diff(&self, state: ClientState) -> Diff               // Changeset for sync

pub fn apply(&self, diff: &Diff)                            // Apply peer changes
pub fn commit(&self)                                         // Finalize changes
```

### Types

```rust
pub enum Type {
    List(NList),
    Map(NMap),
    Text(NText),
    String(NString),
    Atom(NAtom),
}

// List operations
pub fn append(&self, item: impl Into<Type>)
pub fn prepend(&self, item: impl Into<Type>)
pub fn insert(&self, offset: u32, item: impl Into<Type>)
pub fn get(&self, idx: usize) -> Option<Type>
pub fn size(&self) -> u32

// Map operations
pub fn set(&self, key: &str, value: impl Into<Type>)
pub fn get(&self, key: &str) -> Option<Type>
pub fn remove(&self, key: &str)

// Text operations (rich text CRDT)
pub fn insert(&self, offset: u32, item: impl Into<Type>)
pub fn delete(&self, offset: u32)
```

### Diff

```rust
pub struct Diff {
    pub changes: Vec<Change>,  // Low-level CRDT operations
}

pub struct Change {
    pub id: ChangeId { client, clock },
    pub op: /* insert/delete/set */,
    pub parent: Option<ChangeId>,  // Causal dependency
}
```

## Dirty Detection: The Key Innovation

### Problem
State-based sync (send entire doc) is slow. Operation logs are hard to navigate.

### Solution
`dirty_blocks()`: scan change log, return item indices that changed since last sync.

```rust
// Before edits
let v1 = doc.version();  // {client_id → clock}

// Peer makes 3 changes
peer.apply(&diff);

// Know exactly which items changed
let dirty = doc.dirty_blocks(v1);
// Returns: [5, 7, 12]  ← Only 3 items, not 100

// Render only those
for idx in dirty {
    update_ui(get_item(idx));
}
```

**Time:** 6-7 ms for 10,000 items (O(n) scan, but very fast per item).

## Virtual Rendering

For large documents, render only visible items:

```rust
// Render: show items 0-100
fn render_viewport() {
    for i in 0..100 {
        render_item(i);
    }
}

// Update viewport state
let render_time = doc.version();

// Peer edits, we receive diff
doc.apply(&diff);

// Check what changed
let dirty = doc.dirty_blocks(render_time);

// Re-render only changed items in viewport
for idx in dirty {
    if idx < 100 {
        update_item(idx);  // Only if visible
    }
}
```

**Result:** Skip 99% of DOM updates when changes are outside viewport.

## Data Structure

Internal representation:

```
root: List [
  Map {
    "id": Atom("item-0"),
    "text": String("Hello"),
    "children": List [
      Map { ... },
      Map { ... }
    ]
  },
  Map { ... },
  Map { ... }
]

Metadata (not visible to user):
├─ Item ID: (client: 1, clock: 5)   ← Unique, immutable
├─ Parent: (client: 1, clock: 2)    ← Reference to container
├─ Index: 0                         ← Position in list
└─ Tombstone: false                 ← Deleted?
```

## Causal Ordering

Operations form a DAG, not a linear log:

```
Peer A writes: Insert(5, "x")  → ChangeId{A, 1}
Peer B writes: Insert(5, "y")  → ChangeId{B, 1}

Both at same time, both depend on prior state.

DAG:
    ChangeId{A, 1}
         ↓
    Insert("x")
         ↓
    Final state combines both edits
         ↑
    ChangeId{B, 1}
         ↓
    Insert("y")
```

No conflicts: CRDT defines merge semantics.

## Sync Protocol

1. **Doc A** has changes → calls `diff(version_B)`
2. **Creates Diff**: list of changes not in B's state
3. **Send to B**: wire format (compressed)
4. **Doc B** receives → calls `apply(&diff)`
5. **Merges automatically**: CRDT guarantees consistency
6. **Check what changed** → `dirty_blocks(version_before_apply)`

## Performance

| Operation | Time | Items | Notes |
|-----------|------|-------|-------|
| `apply(&diff)` (100 ops) | 2 ms | 100 | Rebuild CRDT tree |
| `dirty_blocks()` (10k items, 5 dirty) | 52 µs | 10,000 | O(changed × depth), not O(n) |
| `has_changes_since()` (quick check) | <1 ms | any | O(1) just scan clock |
| `diff()` (compute changes) | 5 ms | 1000+ | Depends on # changes |
| `get(index)` | <1 µs | any | O(1) list access |
| `list.insert()` | 10 µs | any | Append is O(1) |

## Real-World Usage

### Notion-like block editor

```rust
let doc = Doc::new(Default::default());
let blocks = doc.list();

// Each block is a map
let block = doc.map();
block.set("type", doc.atom("paragraph"));
block.set("content", doc.text());  // CRDT text for rich editing
block.set("children", doc.list()); // Nested blocks
blocks.append(block);

// Peer adds character to a paragraph
let text = block.get("content");
if let Type::Text(t) = text {
    t.insert(5, "new text");
}

// Detect: paragraph content changed
let dirty = doc.dirty_blocks(version);  // [0] ← block 0 dirty
```

### Real-time collaborative spreadsheet

```rust
// Millions of cells, but only a few change
let grid = doc.map();
for row in 0..1000 {
    for col in 0..1000 {
        let cell = doc.map();
        cell.set("value", doc.string("0"));
        grid.set(&format!("{}_{}", row, col), cell);
    }
}

// Peer edits cell [500][500]
let cell = grid.get("500_500");
cell.set("value", doc.string("42"));

// Know exactly which cell to update
let dirty = doc.dirty_blocks(version);  // Very small set
```

## Limitations

- No time-travel (can't query historical state)
- No "undo" (would need operation log + inverse ops)
- Concurrent deletes don't hard-delete (tombstones)
- Memory: full tree in RAM (not streamed)

## See Also

- [carbyne-blocks](../carbyne-blocks) - Materialized block store
- `src/lib.rs` - Core implementation
- `benches/` - Performance benchmarks
- `tests/` - Sync verification

## References

- CRDT theory: https://crdt.tech/
- Operational Transform: https://en.wikipedia.org/wiki/Operational_transformation
- Causal ordering: Lamport clocks + vector clocks
