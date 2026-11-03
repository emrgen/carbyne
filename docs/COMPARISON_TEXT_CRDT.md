# Text CRDT Architecture: Blocktree vs Carbyne

## Executive Summary

**Blocktree** (Go): Append-only RGA text operation log. Server stores ops opaquely; clients reconstruct text via their own RGA replay.

**Carbyne** (Rust): Structural CRDT with embedded text types (NText/NString). Unified transaction model; text is a first-class map/list type.

**Key tradeoff**: Blocktree optimizes for concurrent non-blocking writes (ops commute, no version checks); Carbyne optimizes for read latency (materialized state) and semantic richness (all types are CRDT-aware).

---

## Blocktree (Go) — Append-Only Text Log Architecture

### Design: Per-Block RGA Operation Log

Each block owns a separate log of **RGA text operations**:

```sql
CREATE TABLE crdt_text_ops (
    seq BIGINT PRIMARY KEY AUTO_INCREMENT,
    space_id UUID NOT NULL,
    block_id UUID NOT NULL,
    instance_id UUID NOT NULL,  -- authoritative identity
    kind VARCHAR(32),            -- "insert"|"delete"|"split"|"join"|"mark"
    id_seq VARCHAR(255),         -- base64 Lamport clock from client
    id_site VARCHAR(255),        -- client identity
    payload BLOB NOT NULL,       -- opaque JSON (client RGA wire format)
    applied_version BIGINT,      -- structural transaction counter
    INDEX(space_id, block_id, seq),
    INDEX(space_id, instance_id),
    UNIQUE(space_id, block_id, kind, id_seq, id_site)  -- dedup key
);

CREATE TABLE crdt_instance_placements (
    space_id UUID NOT NULL,
    instance_id UUID NOT NULL,
    block_id UUID NOT NULL,  -- "instance has (or had) a slice in this block"
    PRIMARY KEY(space_id, instance_id, block_id)  -- append-only
);
```

### How It Works

1. **Client sends text ops** without waiting for block version:
   ```
   Client: insert("hello", at 3) → { kind: "insert", payload: {...} }
   Server: append to crdt_text_ops immediately (no version check)
   ```

2. **Server never decodes payload**:
   - Only extracts `(block_id, kind, id_seq, id_site)` for dedup key
   - Payload stays opaque JSON
   - Server forwards raw ops to other clients
   
3. **Client reconstructs text via RGA**:
   ```rust
   // Each client runs TextMirror (their own RGA state machine)
   for op in server_ops {
       text_mirror.apply(op.payload)  // only client decodes
   }
   ```

4. **Split/Join tracked via instances**:
   - When text is split into new block: mark instance_id in both blocks
   - Catch-up query: given blockIDs, widen to instanceIDs, fetch all ops
   ```sql
   SELECT op FROM crdt_text_ops
   WHERE space_id = ? AND instance_id IN (
       SELECT DISTINCT instance_id FROM crdt_instance_placements
       WHERE block_id IN (...)
   ) AND applied_version > last_version
   ```

### Advantages

| Advantage | Why | Impact |
|-----------|-----|--------|
| **Ops commute** | Append-only + RGA semantics | Send without catching up; no conflicts |
| **No version checks on text** | Server just appends | Write: O(1) insert, 1 indexing |
| **Dedup automatic** | (BlockID, Kind, IDSeq, IDSite) unique | Resend safely; duplicate requests idempotent |
| **Split/join correctness** | Instance tracking captures all affected slices | Text moves across blocks; catch-up still finds it |
| **Client flexibility** | Payload opaque | Clients can send any RGA flavor (Diamond Types, YATA, Automerge, etc.) |
| **Minimal server logic** | No RGA interpretation | Smaller attack surface; faster append |

### Disadvantages

| Disadvantage | Why | Impact |
|-------------|-----|--------|
| **Log bloat** | 100k characters = 100k ops = 100k+ rows | Storage: 1–2 GB per 1M blocks × 100k chars |
| **Read replay cost** | Must apply RGA ops to reconstruct text | Latency: 10–50ms for 100k-char document |
| **Separate transactions** | Block ops ≠ text ops | Consistency: block can be "changed" but text not yet written |
| **Instance mapping overhead** | Extra table + widening query | Schema: 3 tables; query: O(log n) + O(k) where k = instances moved |
| **Catch-up complexity** | blockIDs → instanceIDs → ops chain | Clients must understand split/join; harder to reason about |

---

## Carbyne (Rust) — Structural CRDT with Embedded Text

### Design: Text as First-Class Type in Map

Text lives inside the CRDT structure as a **type variant**:

```rust
pub enum Type {
    Atom(NAtom),              // single immutable value
    String(NString),          // mutable string via RGA
    Text(NText),              // rich text + formatting
    List(List),               // ordered container
    Map(Map),                 // key-value container
    // ...
}

pub struct Block {
    id: usize,
    parent_id: Option<usize>,
    table_id: u32,            // immutable table tagging
    block_type: String,
    content: String,          // materialized text (or "[text]" placeholder)
    version: Vec<u8>,         // serialized ClientState
    created_at: i64,
    updated_at: i64,
}
```

### Single Transactional Store

```rust
pub struct Doc {
    pub store: Store,                    // unified CRDT state (all types)
    pub change_store: ChangeStore,       // dirty block tracking
}

impl Doc {
    pub fn dirty_blocks(&self) -> Vec<usize> {
        // O(n) scan of change_store
        // Returns: block IDs that changed since last sync
    }
    
    pub fn get_blocks(&self, dirty_ids: &[usize]) -> Vec<Block> {
        // Materialize only changed blocks
        // Text reconstructed via structural navigation
    }
}
```

### How It Works

1. **Text is embedded in map**:
   ```rust
   let block = doc.map();
   let title_text = doc.text();  // create NText
   block.set("title", title_text.clone());
   
   // Insert is immediate CRDT op
   title_text.insert(0, doc.string("Hello"));
   ```

2. **Single commit applies all changes**:
   ```rust
   block.set("content", content_text);
   block.set("formatted", true);
   doc.commit();  // atomic: both ops + version bump
   ```

3. **Dirty detection identifies changed blocks**:
   ```rust
   let dirty = doc.dirty_blocks();
   // Returns: [123, 456] (only blocks that changed)
   
   for block_id in dirty {
       let block = store.rebuild_block(block_id);
       db.sync(&block)?;
   }
   ```

4. **Materialization preserves text**:
   ```sql
   UPDATE blocks SET
       content = '[text from NText reconstruction]',
       version = ?,
       updated_at = NOW()
   WHERE id = ? AND version = ?;
   ```

### Advantages

| Advantage | Why | Impact |
|-----------|-----|--------|
| **Single transaction** | Block + text in same commit | Consistency: version bump = all changes applied |
| **Structural types** | Text = first-class Type variant | Semantics: can check `if Type::Text(_)` |
| **Dirty detection** | Know exact changed blocks | Efficiency: materialize only what changed |
| **Read latency** | Materialized state | Latency: O(1) to get reconstructed text |
| **Rich nesting** | Text inside map inside list | UX: support "formatted title inside table cell" |
| **No log bloat** | Only final state stored | Storage: 1 row per block, not per keystroke |

### Disadvantages

| Disadvantage | Why | Impact |
|-------------|-----|--------|
| **Text ops require version check** | All ops go through transaction.prepare | Write: extra validation on every insert/delete |
| **Catch-up required** | Must sync block diffs before applying | Write: slower than append-only (O(log n) lookup) |
| **No op commutation** | Concurrent edits may conflict | Resolution: last-write-wins or manual merge |
| **Text extraction missing** | NText is RGA list; full extraction not exposed | Feature gap: no access to raw character RGA IDs |
| **String is opaque** | NString stores "[text]" placeholder | Limitation: can't query raw text content |
| **No split/join semantics** | Text moves with block | Risk: split block = split text; unclear recovery |

---

## Detailed Architecture Comparison

### Storage Model

| Aspect | Blocktree | Carbyne |
|--------|-----------|---------|
| **Text storage** | Append-only ops log (per block) | Embedded in CRDT type (per block) |
| **Schema** | `crdt_text_ops`, `crdt_instance_placements` | `blocks` table (no text table) |
| **Dedup** | (block_id, kind, id_seq, id_site) unique | No dedup (all changes tracked) |
| **History** | Full op log retained | change_store: diffs only |

### Consistency Model

| Aspect | Blocktree | Carbyne |
|--------|-----------|---------|
| **Transaction** | Block ops ≠ text ops (separate) | Block + text in one txn |
| **Version checks** | Block ops only; text ops skip | All ops version-checked |
| **Conflict resolution** | Ops commute (RGA handles it) | Version mismatch = reject & retry |

### Query Performance

| Operation | Blocktree | Carbyne |
|-----------|-----------|---------|
| **Read text** | Replay RGA (10–50ms for 100k chars) | Structural lookup (O(1)) |
| **Catch-up** | blockIDs → instanceIDs → ops query (O(log n) + O(k)) | Diff + apply (O(n) scan) |
| **List blocks by text search** | Scan all ops (slow) | Scan all blocks (faster) |
| **Materialize block** | N/A (server doesn't do it) | O(1) with dirty tracking |

### Developer Experience

| Aspect | Blocktree | Carbyne |
|--------|-----------|---------|
| **API** | "Send ops, server forwards" | "Modify types, commit once" |
| **Reasoning** | "RGA semantics apply" | "CRDT consistency guarantee" |
| **Testing** | RGA replay logic (complex) | Type equality (simple) |
| **Debugging** | Trace ops through RGA | Inspect materialized state |

---

## Synthesis: Optimal Hybrid Design

### Problem with Pure Blocktree
- 100k characters = 100k rows = storage/replay overhead
- Text reconstruction always required (no materialized state)

### Problem with Pure Carbyne
- Text ops require version checks (blocking writes)
- No split/join tracking (text tied to block identity)
- No dedup (every conflicting edit retried)

### Hybrid: Carbyne + Blocktree Patterns

**Layer 1: Structural CRDT (Carbyne)**
- Keep: `Doc { store, change_store }`
- Keep: dirty detection & materialization
- Modification: store text as materialized string (not "[text]" placeholder)

**Layer 2: Text Op Log (Blocktree)**
- Add: separate `crdt_text_ops` table
- Purpose: Audit trail + split/join tracking (not primary text source)
- Ops appended append-only (for dedup)
- Schema:
  ```sql
  CREATE TABLE crdt_text_ops (
      seq BIGINT PRIMARY KEY,
      space_id UUID,
      block_id UUID,
      instance_id UUID,
      kind VARCHAR,           -- "insert"|"delete"|"split"|"join"
      id_seq VARCHAR,         -- Lamport clock
      id_site VARCHAR,        -- client ID
      payload BLOB,           -- opaque JSON
      UNIQUE(space_id, block_id, kind, id_seq, id_site)
  );
  ```

**Layer 3: Instance Tracking (Blocktree)**
- Add: `crdt_instance_placements` for split/join
- Purpose: Correct catch-up when text moves blocks
- Query: `InstancesIn(blockIDs) → instanceIDs → new_blockIDs`

**Write Flow**:
1. Client sends text op
2. Server appends to `crdt_text_ops` (no version check, commutes)
3. Server reconstructs full text (Carbyne-style) for `blocks.content`
4. Single txn updates `blocks` with new content + version
5. Dedup via unique constraint on ops (Blocktree-style)

**Read Flow**:
1. Client requests block
2. Return: materialized `blocks.content` (O(1), Carbyne-style)
3. For catch-up: `GetCrdtTextOpsSince(blockIDs)` (Blocktree-style)
4. Client applies ops via their own RGA (dedup + conflict resolution)

**Benefits**:
- ✅ Write: append-only commuting ops (Blocktree)
- ✅ Read: O(1) materialized text (Carbyne)
- ✅ Dedup: unique constraint + instance tracking
- ✅ Split/join: tracked via instances
- ✅ Single block txn: `blocks` updated atomically

---

## Recommendation

**For Carbyne, adopt Blocktree's text op patterns**:

1. **Keep** structural CRDT + dirty detection
2. **Add** append-only `crdt_text_ops` table for text history
3. **Add** `crdt_instance_placements` for split/join tracking
4. **Materialize** reconstructed text in `blocks.content`
5. **Dedup** via unique constraint on ops

This gives you:
- Write throughput of Blocktree (append-only, no version checks)
- Read latency of Carbyne (materialized state)
- Semantic richness of Carbyne (structured types)
- Correctness of Blocktree (instance tracking)

Estimated implementation: **~200 lines** in carbyne-blocks (text op append + instance tracking in sync).

---

## CORRECTION: For Collaborative Editing, The Hybrid IS REQUIRED

**I was wrong to say "not yet needed".** For ANY collaborative system, ops table is mandatory.

### The Requirements Are Not Optional

#### 1. Multi-client concurrent edits (ALWAYS happens)
- Two users typing simultaneously
- Their position conflicts must resolve deterministically
- RGA ops commute; version checks reject edits
- **Need:** append-only ops table for both to succeed

#### 2. Offline clients (ALWAYS happen)
- Network times out mid-sync
- Client retries same op 3 times
- Without UNIQUE: text gets tripled
- **Need:** UNIQUE(block, kind, id_seq, id_site) dedup constraint

#### 3. Split/join blocks (LIKELY happens)
- User splits paragraph at position 50
- RGA instance continues across blocks
- Server must know both blocks have same instance
- Catch-up only from block 999 must still find ops from block 123
- **Need:** crdt_instance_placements to track slices

#### 4. Audit trail (PROBABLY valuable)
- Users want history: who changed what when
- Compliance requirements (healthcare, legal)
- **Need:** immutable ops log

### Correct Recommendation

**For collaborative editing, adopt the FULL hybrid:**

✓ **Keep** Carbyne's structural CRDT + dirty detection
✓ **Add** append-only `crdt_text_ops` table (NOT optional)
✓ **Add** `crdt_instance_placements` for split/join (NOT optional)
✓ **Add** UNIQUE constraint for dedup (NOT optional)
✓ **Materialize** text in `blocks.content` (O(1) reads)

### Why This Works

**Write Flow:**
1. Client A: insert(0, "A-"), Client B: insert(5, "[B]")
2. Both ops append (different IDs, no conflict)
3. RGA dedup ensures correctness
4. All clients reconstruct same result

**Read Flow:**
1. SELECT blocks.content = O(1) ✓ (Carbyne benefit)
2. Catch-up: GetCrdtTextOpsSince = O(log n) ✓ (Blocktree pattern)

**Durability:**
1. Ops never conflict (append-only)
2. Resends deduped (UNIQUE)
3. Split/join tracked (instances)
4. History preserved (ops log)

**Implementation:** ~200 lines. Essential for correctness.

### Not "Later"—Now

Don't wait until you have problems. Build it from the start:
- Multi-client editing is the whole point
- RGA ops MUST commute (no version checks)
- Offline clients WILL happen (network is real)
- Dedup is non-negotiable (resends happen)

This is **the cost of collaborative editing**. Worth it.
