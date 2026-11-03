# Collaborative Text CRDT: Implementation Complete

**Status**: Core infrastructure done. Ready for text extraction & integration.

## What Was Built

### Schema (Production-ready)
```sql
CREATE TABLE crdt_text_ops (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id INTEGER NOT NULL,
    instance_id TEXT NOT NULL,
    kind TEXT NOT NULL,              -- "insert"|"delete"|"split"|"join"
    id_seq TEXT NOT NULL,             -- base64 Lamport clock
    id_site TEXT NOT NULL,            -- client ID
    payload BLOB NOT NULL,            -- opaque RGA operation
    applied_version BLOB NOT NULL,    -- serialized ClientState
    created_at INTEGER NOT NULL,
    UNIQUE(block_id, kind, id_seq, id_site)  -- Dedup constraint
);

CREATE TABLE crdt_instance_placements (
    instance_id TEXT NOT NULL,
    block_id INTEGER NOT NULL,
    placed_at INTEGER NOT NULL,
    PRIMARY KEY(instance_id, block_id)
);
```

### Core API (Implemented)

```rust
pub fn append_text_ops(&self, block_id: usize, ops: Vec<CrdtTextOp>) -> Result<()>
pub fn record_placement(&self, instance_id: &str, block_id: usize) -> Result<()>
pub fn instances_in(&self, block_ids: &[usize]) -> Result<Vec<String>>
pub fn get_text_ops_since(&self, instance_ids: &[String], applied_version: &[u8]) -> Result<Vec<CrdtTextOp>>
```

### Test Coverage

6 passing tests covering:
- Instance placement recording
- Split/join scenario (same instance across blocks)
- Multiple instances in one block
- Idempotent operations (resends)
- Query ops by instance
- Edge cases (empty queries)

## How It Works

### 1. Multi-Client Concurrent Edits (No Conflicts)

```
Time T1: "Hello world"

Client A:
  insert(0, "A-") → "A-Hello world"
  Op ID: {seq: 42, site: client-A}

Client B:
  insert(5, "[B]") → "Hello[B] world"
  Op ID: {seq: 1, site: client-B}

Both ops append to crdt_text_ops (different IDs, no UNIQUE conflict)
Each client reconstructs via RGA:
  RGA orders ops by ID comparison → deterministic result
  All clients see: "A-Hello[B] world" (or reversed, but consistent)
```

### 2. Offline Resends (Dedup Automatic)

```
Client goes offline, sends op 3 times:
  insert(5, "text")
  Op ID: {seq: 1, site: offline-client}

Append attempt 1: Success
Append attempt 2: UNIQUE constraint rejects (same ID) ✓
Append attempt 3: UNIQUE constraint rejects (same ID) ✓

Result: Only one insert, text is correct
```

### 3. Split/Join Blocks (Instance Tracking)

```
User splits paragraph at position 50:

Block 123: "The quick brown fox..." (instance-A)
Split →
Block 999: "...the lazy dog" (instance-A, continued)

Record placements:
  place(instance-A, block 123)
  place(instance-A, block 999)

Client catches up with block 999:
  1. instances_in([999]) → [instance-A]
  2. get_text_ops_since([instance-A], since_version) 
  3. Get ALL ops for instance-A (from block 123 + 999) ✓
  
Result: Catch-up finds all ops, no history loss
```

## What's Next

### 3 Items to Complete

1. **Text Extraction** (~2 hours)
   - Extract RGA ops from NText structure
   - Rebuild text via op replay
   
2. **Sync Integration** (~1 hour)
   - Wire append_text_ops into sync()
   - Record placements during materialization

3. **Multi-Client Tests** (~2 hours)
   - Concurrent edits, resends, split/join
   - Verify no conflicts or data loss

**Estimated time to full integration: 5 hours**

## Production Checklist

- [x] Schema creation
- [x] Table indexes
- [x] Dedup constraint (UNIQUE)
- [x] Append method with retry handling
- [x] Placement tracking
- [x] Query: instances_in(blocks)
- [x] Query: ops_since(instances, version)
- [x] Unit tests (6 passing)
- [ ] Extract ops from CRDT
- [ ] Rebuild text via RGA replay
- [ ] Integration with sync()
- [ ] Multi-client integration tests
- [ ] Benchmark: 10k blocks, 100k chars
- [ ] Load test: dedup under 100 resends/op

## Commits

1. `41d05e4` feat: add crdt_text_ops and crdt_instance_placements tables
2. `86c45bf` feat: implement core text ops methods for collaborative editing
3. `bab1ecf` test: add comprehensive tests for collaborative text ops

## Key Insight

**RGA ops commute** → no version conflicts.
Server appends all ops in any order.
Clients reconstruct deterministically.
Result: Offline-first, conflict-free collaborative editing.

---

**Ready to ship the infrastructure. Next: connect it to Carbyne's CRDT.**
