# Carbyne as a document service: storage & ops cost audit

Scope: `crates/carbyne` + `crates/carbyne-blocks`, server holds docs, clients sync via `diff()`/`apply()`.
Evidence is `file:line` at time of audit. "Impact" = what it does to the bill.

## Findings, ranked by cost impact

| # | Problem | Evidence | Impact |
|---|---------|----------|--------|
| 1 | `dirty_blocks()` returns **every** root block when any change exists; root scan ignores `changed_since` | `crates/carbyne/src/doc.rs:177-182` | `BlockStore::sync` rewrites all N rows per sync. Write amplification ≈ N× per edit. Dominant ops cost. |
| 2 | No tombstone GC. `DeleteItem` kept forever, deleted `ItemData` kept forever | `item.rs:178-184`, `store.rs:686-698` | CRDT state grows monotonically; "1.1 GB CRDT ops" never shrinks. **Tier 1 done, see below.** |
| 3 | No CRDT persistence. `DocStoreData::from_diff` is empty; `persist.rs` encode exists but nothing writes it. `DiffStore::get` ignores `state`, returns whole doc | `persist.rs:41-45`, `diffstore.rs:35-39` | Every `Doc` must live in RAM (`Arc<Doc>`, `carbyne-blocks/src/lib.rs:42-46`). RAM is the bill. |
| 4 | Archive pipeline is fake: `find_old_sessions` returns `[]`, `upload_session` only formats a string, `total_bytes = count * 500` | `archive_job.rs:153-164, 144`, `s3_backend.rs:69-76` | `ARCHIVE_COMPRESS_SUMMARY.md` "72% savings" is unverified. Nothing leaves hot SQLite. |
| 5 | `blocks.version` stores bincode of full `ClientState` per row | `carbyne-blocks/src/lib.rs:230` | Vector clock × every row: 8·C bytes × N rows duplicated. |
| 6 | Per-row `SELECT 1` exists check then INSERT/UPDATE | `lib.rs:98-111` | 2 statements/row. `INSERT … ON CONFLICT DO UPDATE` is 1. |
| 7 | No `PRAGMA journal_mode=WAL`, `synchronous=NORMAL` | `lib.rs:60-71`, `storage.rs:29-51` | Rollback journal + fsync per txn. 2–10× slower writes, more IOPS. |
| 8 | Diff wire format uncompressed, fixed big-endian `u32` for ids/clocks/lengths | `codec_v1.rs:64-90`, `id.rs:359-362` | Clocks are small ints; varint + zstd cuts diff bytes 3–5×. Egress. |
| 9 | `Content::Types` / `Content::Embed` encode **nothing** | `item.rs:1337-1342` | Correctness, not cost: nested-type diffs silently lose data. Must fix before any GC tier ≥2. |
| 10 | `diff()` scans all items per client, clones `ItemData` | `store.rs:1100-1116` | O(total items) per client sync, not O(delta). CPU ∝ doc size × client count. |

Found during implementation (pre-existing, not fixed here):

- `Diff` wire round-trip panics `decoder: out of bounds` (`codec_v1.rs:155`) for any doc with a committed change. Existing `diff.rs::test_encode_decode_diff` only covers uncommitted root fields. Item #8 work must start here.
- Remote deletes were never applied: `tx.rs:174-183` shadowed `store` with the per-client delete store, so `store.contains(&id)` always looked in the wrong map and tombstones never reached `ready`. Fixed below as a prerequisite.
- `carbyne-blocks` does not compile on HEAD (`E0062: field created_at specified more than once`, 10 errors). Independent of core crate changes.

## Tombstone GC: when it is safe

Safe point = **causal stability**: a tombstone may go only when no op that could still arrive references it.

Why not "on delete":
- Deleted `ItemData` is still the `left_id`/`right_id` origin for ops from clients that saw it before deletion. Encoder drops `parent_id` when `left_id` present (`codec_v1.rs:259-295`), so the left tombstone is the *only* route to the parent.
- `apply()` undoes/redoes change history to order movers (`doc.rs:251-265`); a delete change below the sender's deps can be undone, resurrecting the item.
- `pending` store (`store.rs:133`) holds items waiting for deps; their targets must exist.
- `diff()` to a stale client ships deleted items and splits ranges by tick (`store.rs:1100-1112`); needs id + `ticks()` at minimum.

### Tier 1 — coalesce (DONE)
Merge runs of consecutive delete ops whose targets are adjacent into one tombstone. Pure index compaction; no id is dropped, every clock is still representable. Runs on `commit()`, no coordination.

### Tier 2 — drop content, keep skeleton
Replace `Content::String/Binary` with `Content::Null` on deleted items; keep `id`, `kind`, `left_id/right_id/parent_id`, `ticks`.
Condition: delete change ≤ **stable frontier** = per-client `min(ack[c] for c in live_clients)`. Below it, no incoming change can have older deps, so DAG undo never reaches it and no pending op can target it.

### Tier 3 — physical removal
Same frontier condition plus:
- prune `changes` + `dag` below frontier (else `store.changes.get(id).unwrap()` at `doc.rs:224` panics on dep lookup),
- `disconnect()` from linked list and relink neighbours,
- bump a **doc epoch**; any client presenting a `ClientState` older than the GC frontier gets a full snapshot, not a diff. `DiffStore::get` must check this.

### Prerequisites for tier 2/3 (none exist today)
1. Server-side `ack[client] = ClientState` per doc, persisted.
2. Liveness policy: client silent > N days → evicted from `live_clients`, forced resync on return. N = 7–30 days.
3. Frontier floor: `pending` must be empty for that client column.
4. Fix item #9 first, or "dropped by GC" is indistinguishable from "never encoded".

## Tier 1 implementation

Files: `crates/carbyne/src/delete.rs`, `store.rs`, `tx.rs`, `sync.rs` (test).

- `DeleteItem` gains `ticks: u32` (delete-op span; `1` for a fresh delete). `end_clock()` = `id.clock + ticks - 1`. `join()` folds the next consecutive delete op of the same client when target ranges touch on either side.
- Wire: `DeleteItem` now encodes `IdRange(id.client, id.clock, end_clock)` + target range (was `Id` + range). Same byte count for `ticks == 1` (two `u32` + size). Not backward compatible with pre-change encoded blobs; nothing persists them yet (item #3).
- `DeleteItemStore::coalesce(range)` (`store.rs`): walks the change's delete ops in clock order, joins adjacent, rewrites the per-client map. Called at the end of `DocStore::commit()` after deps are computed.
- `ItemStore<DeleteItem>::diff` now tests `end_clock() > clock` so a coalesced tombstone still ships to a peer that saw only its head.
- Prerequisite fix in `tx.rs`: `prepare()` shadowing bug; `apply()` now marks every item in a tombstone's target range deleted and records the tombstone. Before this, remote deletes silently no-oped.

Test: `sync::test::test_commit_coalesces_adjacent_tombstones` — 3 adjacent deletes → 1 tombstone, syncs to a peer, peer sees 2 of 5 atoms, `DeleteItemStore` round-trips through `EncoderV1`.

Result: `cargo test -p carbyne` 109 passed.

Ceiling: coalescing only within one commit, only consecutive ops with touching targets. Typical editor delete-selection hits this; scattered deletes don't. Cross-commit coalescing needs change-id rewriting; not worth it before tier 2.

## Next steps, in cost order

1. Fix #1 (`dirty_blocks` filter by `changed_since`): one-line scope fix, largest ops win.
2. Fix #7 + #6 (WAL, upsert): config + one statement, no schema change.
3. Fix the `Diff` round-trip panic, then #8 (varint + zstd on the wire).
4. Fix #3: actually persist `DocStoreData`; unlocks unloading idle docs from RAM and tier 2/3 GC.
5. Delete the fake archive code (#4) or implement it; today it misleads.
