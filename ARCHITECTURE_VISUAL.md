# Text CRDT: Blocktree vs Carbyne (Visual)

## Blocktree: Append-Only Ops Log

```
┌─────────────────────────────────────────────────────┐
│  Client                                             │
│  ┌─────────────────────────────────────┐           │
│  │ TextMirror (RGA state machine)      │           │
│  │                                     │           │
│  │ insert(3, "hello")                  │           │
│  │ → { kind: "insert",                 │           │
│  │     id: { seq: 42, site: "client1"} │           │
│  │     payload: { pos: 3, text: "..." }│           │
│  │   }                                 │           │
│  └─────────────────────────────────────┘           │
└─────────────────────────────────────────────────────┘
                      ↓ send op
┌─────────────────────────────────────────────────────┐
│  Server                                             │
│  ┌────────────────────────────────────┐            │
│  │ AppendCrdtTextOpsBatch             │            │
│  │ 1. Check dedup key:                │            │
│  │    (block_id, kind, seq, site)     │            │
│  │    UNIQUE constraint               │            │
│  │ 2. Append to crdt_text_ops         │            │
│  │ 3. Record placement in             │            │
│  │    crdt_instance_placements        │            │
│  │ 4. NEVER decode payload            │            │
│  └────────────────────────────────────┘            │
│          ↓                                          │
│  ┌──────────────────────────────────┐             │
│  │ Database                         │             │
│  │ ┌──────────────────────────────┐ │             │
│  │ │ crdt_text_ops                │ │             │
│  │ │ ┌──────────────────────────┐ │ │             │
│  │ │ │ seq  1: insert, pos=0    │ │ │             │
│  │ │ │ seq  2: insert, pos=5    │ │ │             │
│  │ │ │ seq  3: delete, id=...   │ │ │             │
│  │ │ │ seq  4: split, new_bk=999│ │ │             │
│  │ │ │ ...  (append-only!)      │ │ │             │
│  │ │ └──────────────────────────┘ │ │             │
│  │ ├──────────────────────────────┤ │             │
│  │ │ crdt_instance_placements     │ │             │
│  │ │ instance_id  → block_id      │ │             │
│  │ │ uuid-abc-123 → 999 (moved)   │ │             │
│  │ │ uuid-abc-123 → 42 (origin)   │ │             │
│  │ └──────────────────────────────┘ │             │
│  └──────────────────────────────────┘             │
└─────────────────────────────────────────────────────┘
                      ↓ send ops to other clients
┌─────────────────────────────────────────────────────┐
│  Other Client                                       │
│  ┌─────────────────────────────────┐               │
│  │ TextMirror.apply(op.payload)    │               │
│  │ → reconstructs text via RGA     │               │
│  │ "hello" (100k ops = long replay)│               │
│  └─────────────────────────────────┘               │
└─────────────────────────────────────────────────────┘

KEY: ✓ Append-only (no updates/deletes)
     ✓ Ops commute (RGA semantics)
     ✗ Log bloat (100k ops = 100k rows)
     ✗ Read cost (replay RGA)
```

---

## Carbyne: Structural CRDT

```
┌──────────────────────────────────────────────────┐
│  Client                                          │
│  ┌──────────────────────────────────┐           │
│  │ Doc {                            │           │
│  │   store: CRDT,                   │           │
│  │   change_store: DirtyTracker     │           │
│  │ }                                │           │
│  │                                  │           │
│  │ let title = doc.text();          │           │
│  │ title.insert(0, "Hello");        │           │
│  │                                  │           │
│  │ let block = doc.map();           │           │
│  │ block.set("title", title);       │           │
│  │ block.set("version", 42);        │           │
│  │                                  │           │
│  │ doc.commit();  ← atomic!         │           │
│  └──────────────────────────────────┘           │
└──────────────────────────────────────────────────┘
              ↓ sync to other Doc
┌──────────────────────────────────────────────────┐
│  Server (BlockStore)                             │
│  ┌──────────────────────────────────┐            │
│  │ dirty_blocks() → [123, 456]      │            │
│  │ returns changed block IDs        │            │
│  │                                  │            │
│  │ rebuild_block(123)               │            │
│  │ → walks CRDT structure           │            │
│  │ → extracts "Hello" from NText    │            │
│  └──────────────────────────────────┘            │
│          ↓                                       │
│  ┌──────────────────────────────────┐            │
│  │ Database                         │            │
│  │ ┌──────────────────────────────┐ │            │
│  │ │ blocks                       │ │            │
│  │ │ ┌──────────────────────────┐ │ │            │
│  │ │ │ id: 123                  │ │ │            │
│  │ │ │ content: "Hello"         │ │ │            │
│  │ │ │ version: 42              │ │ │            │
│  │ │ │ updated_at: 2026-11-03   │ │ │            │
│  │ │ └──────────────────────────┘ │ │            │
│  │ │ (materialized state)         │ │            │
│  │ └──────────────────────────────┘ │            │
│  └──────────────────────────────────┘            │
└──────────────────────────────────────────────────┘
              ↓ read block
┌──────────────────────────────────────────────────┐
│  Client                                          │
│  ┌──────────────────────────────────┐           │
│  │ block = db.get_block(123)        │           │
│  │ // content: "Hello" (O(1)!)      │           │
│  │ // render it                     │           │
│  └──────────────────────────────────┘           │
└──────────────────────────────────────────────────┘

KEY: ✓ Single transaction (all ops atomic)
     ✓ Materialized read (O(1))
     ✓ Dirty detection (only materialize changes)
     ✗ Write needs version check (slower)
     ✗ No dedup (concurrent edits conflict)
     ✗ No split/join tracking
```

---

## Hybrid: Best of Both (RECOMMENDED)

```
┌──────────────────────────────────────────────────────┐
│  Client                                              │
│  ┌──────────────────────────────────┐               │
│  │ TextMirror                       │               │
│  │                                  │               │
│  │ insert(0, "Hello")               │               │
│  │ → {kind: "insert",               │               │
│  │    id: {seq, site},              │               │
│  │    payload: {...}}               │               │
│  └──────────────────────────────────┘               │
│           ↓                                         │
│  ┌──────────────────────────────────┐               │
│  │ Doc (Carbyne-style)              │               │
│  │ apply(text_op) →                 │               │
│  │ reconstruct full text             │               │
│  └──────────────────────────────────┘               │
└──────────────────────────────────────────────────────┘
                 ↓ send op + reconstructed text
┌──────────────────────────────────────────────────────┐
│  Server                                              │
│  ┌────────────────────────────────────────────────┐ │
│  │ AppendCrdtTextOpsBatch                         │ │
│  │ 1. Append op to crdt_text_ops                  │ │
│  │    (unique: block_id, kind, seq, site)         │ │
│  │ 2. Record instance placement                   │ │
│  │ 3. Reconstruct full text (RGA replay)          │ │
│  │ 4. UPDATE blocks.content with text             │ │
│  │ 5. Single transaction! (atomic)                │ │
│  └────────────────────────────────────────────────┘ │
│           ↓                                        │
│  ┌────────────────────────────────────────────────┐ │
│  │ Database                                       │ │
│  │ ┌──────────────────────────────────────────┐   │ │
│  │ │ crdt_text_ops (append-only)              │   │ │
│  │ │ seq 1: insert, payload={...}             │   │ │
│  │ │ seq 2: delete, id={seq, site}            │   │ │
│  │ │ UNIQUE(block_id, kind, seq, site)        │   │ │
│  │ └──────────────────────────────────────────┘   │ │
│  │ ┌──────────────────────────────────────────┐   │ │
│  │ │ crdt_instance_placements (tracking)      │   │ │
│  │ │ instance_id → block_id (for split/join)  │   │ │
│  │ └──────────────────────────────────────────┘   │ │
│  │ ┌──────────────────────────────────────────┐   │ │
│  │ │ blocks (materialized)                    │   │ │
│  │ │ id: 123, content: "Hello", version: 42  │   │ │
│  │ └──────────────────────────────────────────┘   │ │
│  └────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
           ↓ read block               ↓ catch-up
      ┌──────────────┐           ┌──────────────┐
      │ blocks.content│          │ crdt_text_ops│
      │ "Hello" (O(1))│          │ apply ops    │
      └──────────────┘           └──────────────┘

KEY: ✓ Write: append-only + commuting (no version check)
     ✓ Read: materialized content (O(1))
     ✓ Dedup: unique constraint on ops
     ✓ Consistency: single transaction (blocks + ops + placements)
     ✓ Split/join: tracked via instances
```

---

## Storage & Query Cost Comparison

```
For 1M blocks, 100 characters average text, 10 years history

BLOCKTREE (Append-Only)
├─ Assumption: 100 chars = 100 edits average (aggressive RGA)
│  
├─ crdt_text_ops
│  └─ 1M blocks × 100 ops × 300 bytes = 30 GB
│
├─ crdt_instance_placements
│  └─ 1M blocks × 10 splits = 10M rows × 50 bytes = 500 MB
│
├─ blocks (no text)
│  └─ 1M rows × 100 bytes = 100 MB
│
├─ Total: ~30.6 GB ❌ (bloated by text ops)
│
└─ Read text:
   └─ Replay 100 ops (RGA) = 10–50ms ❌ (slow)

CARBYNE (Materialized)
├─ blocks with content
│  └─ 1M rows × (200 + 100) bytes = 300 MB
│
├─ change_store (diffs only)
│  └─ ~100 MB (recent changes)
│
├─ Total: ~400 MB ✓ (compact)
│
└─ Read text:
   └─ SELECT content = O(1) ✓ (fast)

HYBRID (Carbyne + Blocktree Patterns)
├─ blocks (materialized)
│  └─ 1M × 200 bytes = 200 MB
│
├─ crdt_text_ops (audit trail only, sampled)
│  └─ 1M blocks × 10 ops (not 100) = 100M rows × 200 bytes = 20 GB
│  └─ (compress JSON payload + archive old ops) = 2 GB
│
├─ crdt_instance_placements
│  └─ 500 MB
│
├─ Total: ~3 GB ✓ (70% smaller than Blocktree)
│
└─ Benefits:
   ├─ Read text: O(1) ✓
   ├─ Write: append-only ✓
   ├─ Dedup: UNIQUE constraint ✓
   ├─ Split/join: tracked ✓
   └─ Audit: full history available ✓
```

---

## Timeline: Adopt Blocktree Patterns in Carbyne

**Phase 1 (2–3 days)**: Add text ops logging
```
├─ Create crdt_text_ops table
├─ Append ops in sync()
├─ Unique constraint for dedup
└─ Test: ops appended, dedup works
```

**Phase 2 (2–3 days)**: Add instance tracking
```
├─ Create crdt_instance_placements table
├─ Record placements on split/join
├─ Query: InstancesIn(blockIDs)
└─ Test: catch-up finds all ops after split
```

**Phase 3 (1–2 days)**: Materialize text
```
├─ Reconstruct full text in rebuild_block()
├─ Store in blocks.content column
├─ Update sync() to save reconstructed text
└─ Test: read latency O(1), ops still logged
```

**Estimated**: ~200 lines of code, 1 week total.
