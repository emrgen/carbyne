# Text CRDT: Next Steps for Carbyne

## Current State

Carbyne has:
- ✅ Structural CRDT (Map, List, Text, String, Atom)
- ✅ Single transactional store (atomic commits)
- ✅ Dirty block detection (change_store)
- ✅ Materialization to BlockStore (SQLite)
- ✅ Table tagging (immutable table_id)

Blocktree has:
- ✅ Append-only text ops log (crdt_text_ops)
- ✅ Instance tracking for split/join (crdt_instance_placements)
- ✅ Dedup via UNIQUE constraint
- ✅ Commuting ops (no version conflicts)

## Problem: Text Scaling

**Current Carbyne**: NText is embedded in map
- Read: O(1) ✓
- Write: must version-check block
- Storage: efficient (no op log bloat)
- Issue: no split/join tracking; no dedup on concurrent edits

**Current Blocktree**: RGA ops log
- Read: O(n) replay ❌
- Write: O(1) append (commutes) ✓
- Storage: bloated (100k ops = 100k rows = 30 GB)
- Benefit: split/join tracked; dedup automatic

## Proposed Hybrid Solution

### 1. Add Append-Only Text Ops Table

```sql
CREATE TABLE crdt_text_ops (
    seq BIGINT PRIMARY KEY AUTO_INCREMENT,
    space_id UUID NOT NULL,
    block_id UUID NOT NULL,
    instance_id UUID NOT NULL,
    kind VARCHAR(32),              -- "insert"|"delete"|"split"|"join"
    id_seq VARCHAR(255),           -- Lamport clock (base64)
    id_site VARCHAR(255),          -- client ID
    payload BLOB NOT NULL,         -- opaque JSON (RGA wire format)
    applied_version BIGINT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    -- Append-only + dedup
    UNIQUE(space_id, block_id, kind, id_seq, id_site),
    INDEX(space_id, block_id),
    INDEX(space_id, instance_id),
    INDEX(space_id, applied_version)
);
```

**Purpose**: 
- Audit trail (optional, can be archived)
- Dedup key for resent ops
- Split/join tracking (see next)

### 2. Add Instance Placement Tracking

```sql
CREATE TABLE crdt_instance_placements (
    space_id UUID NOT NULL,
    instance_id UUID NOT NULL,
    block_id UUID NOT NULL,
    placed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY(space_id, instance_id, block_id)
);
```

**Purpose**:
- Track which text instances were placed in which blocks
- Survives split/join (instance may have slices in multiple blocks)
- Enables correct catch-up: blockIDs → instanceIDs → all ops

**Example**:
```
Block 123: instance-A (original block)
Split → new block 999
Block 999: instance-A (same instance, moved)

crdt_instance_placements:
  (instance-A, block 123)
  (instance-A, block 999)  ← new placement recorded

Catch-up request: blockIDs=[123]
Query: SELECT DISTINCT instance_id FROM crdt_instance_placements 
       WHERE block_id IN (123)
  → instance-A
Then: SELECT * FROM crdt_text_ops 
      WHERE instance_id = 'instance-A' AND applied_version > last_version
  → finds ops that moved to block 999!
```

### 3. Materialize Text in Blocks Table

Keep:
```sql
ALTER TABLE blocks ADD COLUMN content TEXT;  -- materialized text
```

**During sync()**:
```rust
pub fn sync(&self, dirty_ids: &[usize]) -> Result<()> {
    for block_id in dirty_ids {
        // 1. Reconstruct full text via NText RGA
        let text = self.rebuild_text(block_id)?;
        
        // 2. Append ops to crdt_text_ops
        for op in &self.extract_text_ops(block_id) {
            db.append_text_op(block_id, op)?;
            db.record_placement(op.instance_id, block_id)?;
        }
        
        // 3. Update blocks table (single transaction)
        db.update_block(block_id, &Block {
            content: text,
            version: self.block_version(block_id),
            updated_at: now(),
            ..
        })?;
    }
    Ok(())
}
```

**Benefits**:
- Read text: O(1) from blocks.content ✓
- Write ops: append-only (no version check) ✓
- Dedup: UNIQUE(block_id, kind, id_seq, id_site) ✓
- Split/join: tracked via instance placements ✓
- Single txn: blocks + ops + placements atomic ✓

### 4. Implement Text Op Extraction

```rust
// In rebuild_block() or new method extract_text_ops()
fn extract_text_ops(&self, block_id: usize) -> Vec<CrdtTextOp> {
    let text = self.doc.store.get_block(block_id);
    
    // Walk NText RGA items
    text.text()
        .item_ref()
        .borrow()
        .items()
        .iter()
        .filter_map(|item| {
            match item.origin() {
                Some(id) => Some(CrdtTextOp {
                    block_id,
                    instance_id: self.instance_id_for(id),
                    kind: if item.deleted() { "delete" } else { "insert" },
                    id_seq: id.seq.to_base64(),
                    id_site: id.site.to_string(),
                    payload: serde_json::to_vec(&item.content()).ok()?,
                }),
                None => None,
            }
        })
        .collect()
}
```

## Implementation Timeline

### Week 1: Schema & Append
- Day 1–2: Add crdt_text_ops + crdt_instance_placements tables
- Day 2–3: Implement AppendCrdtTextOpsBatch (idempotent append)
- Day 3: Write tests (dedup, idempotence, ordering)
- Day 4–5: Test split/join placement recording

### Week 2: Materialization & Extraction
- Day 1–2: Implement extract_text_ops() + rebuild_text()
- Day 2–3: Update sync() to append ops + materialize text
- Day 3–4: Test catch-up queries (blockIDs → instanceIDs → ops)
- Day 4–5: Benchmark (10k blocks, 100k chars, concurrent edits)

### Week 3: Integration & Optimization
- Day 1: Archive old ops (compress, move to cold storage)
- Day 2: Optimize queries (indexes, prepared statements)
- Day 3–4: Load testing (10M ops/day, check latency)
- Day 5: Documentation + examples

## Expected Metrics

| Metric | Before (Carbyne) | After (Hybrid) |
|--------|------------------|----------------|
| **Read latency** | O(1) | O(1) (no change) |
| **Write latency** | O(log n) (version check) | O(1) (append-only) |
| **Storage (100k chars)** | 500 bytes | 500 bytes + ops log |
| **Storage (1M blocks)** | ~500 MB | ~3 GB (with ops) |
| **Ops log size** | N/A | ~30 bytes per op |
| **Dedup** | None | UNIQUE constraint |
| **Split/join tracking** | None | crdt_instance_placements |

## Open Questions

1. **Payload opaque vs decoded?**
   - Blocktree keeps payload opaque JSON (clients decode)
   - Carbyne could decode + store structured ops
   - Trade: storage vs parser safety

2. **Archive old ops?**
   - Keep ops forever: 30+ GB per year
   - Archive to cold storage: 1 GB compressed
   - Recommendation: archive ops older than 3 months

3. **Client-side RGA or server-side?**
   - Blocktree: each client has TextMirror (RGA)
   - Carbyne: could use Automerge/Yjs on client
   - Recommendation: client-side for conflict resolution

4. **Full-text search?**
   - Index blocks.content for keyword search
   - No need for op log queries (materialized state)
   - Recommendation: full-text index on blocks.content

## Start With

1. Add tables (1 day)
2. Implement append (1 day)
3. Add tests (1 day)
4. Ship v1 (optional ops log, dedup only)
5. Iterate: instance tracking → split/join support


---

## CLARIFICATION: Do You Actually Need crdt_text_ops?

**Short answer: No, not for current Carbyne.**

The hybrid design is useful when:
1. Text split/joins across blocks (RGA instances span multiple blocks)
2. Multi-client concurrent edits on same text (need dedup)
3. Offline clients (resend detection)
4. Compliance audit trail (log every keystroke)

**Current Carbyne (EMRGen) has none of these:**
- ✓ Each block owns its text (no sharing/split-join)
- ✓ Single/few editors (no concurrent conflicts)
- ✓ Materialized state is enough (no need for op log)
- ✓ Block version number is sufficient consistency guarantee

**So keep it simple:**
```
Client edits:
  block.set("title", "hello") → doc.commit()

Server sync:
  dirty_blocks() → [123]
  rebuild_text(123) → "hello"
  UPDATE blocks SET content="hello", version=?
  ✓ Done

No ops table needed.
```

**When to add ops table:**
Only if you later need:
- Multi-client text editing (conflicts need RGA dedup)
- Text split/join at RGA level (instances across blocks)
- Offline-first (resend detection)
- Audit compliance (keystroke history)

Until then: Carbyne's current model is already optimal.
