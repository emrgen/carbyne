# Carbyne as a document service: storage & ops cost audit

Scope: `crates/carbyne`, `crates/carbyne-blocks`, `crates/carbyne-db`. Server holds docs, clients sync via `diff()`/`apply()`.
Evidence is `file:line` at time of audit. "Impact" = effect on the bill.

Related docs in `../cbdocs/` (read, reconciled below):
- `db/README.md` — target architecture: op blocks, group commit, actor/clock index, object-storage tiering.
- `db/cost/README.md` — planning estimates (8× history, 3× write bytes, 100× tiny-sync read work). Not measured.
- `storage/README.md` — seekable history format, VV extraction, retention floor.
- `perf/README.md` — ranked source gaps (P0: whole-history scans in diff/change/persist).
- `CARBYNE_DB_*.md` — carbyne-db crate status.

## Where the code is vs. where the docs say it should be

| Layer               | cbdocs target                                                                                               | Code today                                                                                                                                                                                                                                                                                                                                            | Gap                                                                |
| ------------------- | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| History persistence | Actor-local immutable blocks, clock-range index, sealed → object storage (`db/README.md` §Operation blocks) | Nothing. `persist.rs:41-45` `from_diff` is empty; `DiffStore::get` ignores `state` (`diffstore.rs:35-39`); carbyne-db `Op = ()` (`changelog.rs:8`), `load()` returns `Doc::default()` (`changelog.rs:47-53`)                                                                                                                                          | Whole layer missing. Every doc lives in RAM.                       |
| Durable unit        | Group commit, ≤1 fsync per 10–100 changes (`db/cost` §Target budgets)                                       | carbyne-db `apply()` does `append` (read-modify-write of the whole op list, `changelog.rs:34-43`) **then** `snapshot()` which deletes the log (`instance.rs:76-90`, `changelog.rs:57-62`). Net: 2 RocksDB writes + 1 delete per op, log never holds more than one batch. carbyne-blocks: no WAL pragma, per-row exists-check (`lib.rs:60-71, 98-111`) | Opposite of design: 3 durable ops per logical op, zero batching.   |
| Tiny / no-op sync   | `O(actor count + manifest)` (`db/cost` §Selective sync)                                                     | `ItemStore::diff` scans + clones every item per client (`store.rs:1100-1116`); `ChangeStore::diff` scans every change (`change.rs:458-475`)                                                                                                                                                                                                           | `O(retained history)`. perf P0, unchanged.                         |
| Materialized blocks | Incremental current-state indexes (`db/README.md` §Current-state indexes)                                   | `dirty_blocks()` returns every root block when anything changed (`doc.rs:177-182`); `BlockStore::sync` rewrites all N rows                                                                                                                                                                                                                            | Write amplification ≈ N× per edit. Dominant ops cost today.        |
| Retention / GC      | Retained-history floor + snapshot-required protocol (`storage/README.md` §Compaction and retention)         | No GC, no floor, no epoch. **Tier 1 coalesce done** (below).                                                                                                                                                                                                                                                                                          | Tier 2/3 blocked on ack tracking.                                  |
| Wire encoding       | Varints, delta refs, implicit clocks, block compression (`db/README.md` §Actor-local encoding)              | Fixed BE `u32` everywhere (`codec_v1.rs:64-90`), no compression. `Content::Types`/`Embed` encode nothing (`item.rs:1337-1342`). `Diff` round-trip panics for any committed change (`codec_v1.rs:155`)                                                                                                                                                 | Codec must be fixed before it can be a disk format.                |
| Archive to S3       | Sealed blocks → object storage with checksum verify (`db/README.md` §Moving sealed history)                 | `find_old_sessions` returns `[]`, `upload_session` formats a string, `total_bytes = count*500` (`archive_job.rs:153-164,144`, `s3_backend.rs:69-76`)                                                                                                                                                                                                  | `ARCHIVE_COMPRESS_SUMMARY.md` "72% savings" not real.              |
| Sync protocol       | VV-based extraction with fixed target VV (`storage/README.md` §VV-based extraction)                         | carbyne-db uses a single `u64` lamport per instance (`instance.rs`, `message.rs`). CAVEATS.md §2 flags this. Core crate already has per-client `ClientState` VV (`state.rs:72-75`)                                                                                                                                                                    | carbyne-db picked the wrong clock; core already has the right one. |

Summary: cbdocs design is sound and matches this audit. The crate that was supposed to implement it (`carbyne-db`) is placeholder-level and its one real write path is more expensive than doing nothing. Nothing in cbdocs is wrong; nothing in cbdocs is built.

## Findings, ranked by cost impact

| #  | Problem                                                                    | Evidence                                                      | Impact                                                                                                           |
| -- | -------------------------------------------------------------------------- | ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| 1  | `dirty_blocks()` ignored `changed_since`, returned all root blocks         | `doc.rs:177-182` (old)                                        | N row rewrites per edit. **DONE**, see below.                                                                    |
| 2  | carbyne-db `apply()`: read whole log, rewrite, then snapshot+delete per op | `changelog.rs:34-43,57-62`, `instance.rs:76-90`               | 3 durable writes/op, log read ∝ batch size. Must become append-only + periodic snapshot.                         |
| 3  | No tombstone GC                                                            | `item.rs:178-184`, `store.rs:686-698`                         | Monotonic growth. Tier 1 done.                                                                                   |
| 4  | No CRDT persistence anywhere                                               | `persist.rs:41-45`, `diffstore.rs:35-39`, `changelog.rs:8,47` | All docs resident. RAM is the bill (`db/cost` §Resident memory: 10k docs × 2 MiB = 20 GiB vs 200 MiB hot cache). |
| 5  | Archive pipeline is a stub                                                 | `archive_job.rs:153-164`, `s3_backend.rs:69-76`               | No cold tier.                                                                                                    |
| 6  | `blocks.version` = bincode `ClientState` per row                           | `carbyne-blocks/src/lib.rs:230`                               | 8·C bytes × N rows.                                                                                              |
| 7  | Per-row `SELECT 1` then INSERT/UPDATE                                      | `lib.rs:98-111`                                               | 2× statements.                                                                                                   |
| 8  | No `journal_mode=WAL` / `synchronous=NORMAL`                               | `lib.rs:60-71`, `storage.rs:29-51`                            | fsync per txn.                                                                                                   |
| 9  | Fixed-width uncompressed codec; `Diff` round-trip panics                   | `codec_v1.rs:64-90,155`                                       | 3–5× egress; cannot persist as-is.                                                                               |
| 10 | `Content::Types`/`Embed` encode nothing                                    | `item.rs:1337-1342`                                           | Silent data loss. Blocks GC tier ≥2 and any disk format.                                                         |
| 11 | `diff()` scans all items + changes per client                              | `store.rs:1100-1116`, `change.rs:458-475`                     | O(history) per sync. perf P0.                                                                                    |
| 12 | carbyne-db lamport `u64` instead of VV                                     | `instance.rs`, `message.rs`                                   | Cannot express per-actor coverage; dedup (CAVEATS §8) unsolvable with it.                                        |

Pre-existing, found while implementing, not fixed:
- `Diff` wire round-trip panics `decoder: out of bounds` (`codec_v1.rs:155`) for any committed change. `diff.rs::test_encode_decode_diff` only covers uncommitted root fields.
- Remote deletes never applied: `tx.rs:174-183` shadowed `store`. **Fixed** as a prerequisite for tier 1.
- `carbyne-blocks` does not compile on HEAD (`E0062` duplicate fields, 10 errors).

## Tombstone GC: when it is safe

Safe point = **causal stability**: a tombstone may go only when no op that could still arrive references it. Matches `storage/README.md` §Correctness: "Dropping operations, tombstones, or causal metadata may not [preserve sync semantics]."

Why not "on delete":
- Deleted `ItemData` is the `left_id`/`right_id` origin for ops from clients that saw it. Encoder drops `parent_id` when `left_id` present (`codec_v1.rs:259-295`); the tombstone is the only route to the parent.
- `apply()` undoes/redoes history to order movers (`doc.rs:251-265`); a delete change below the sender's deps can be undone.
- `pending` (`store.rs:133`) holds items waiting for deps; targets must exist.
- `diff()` to a stale client ships deleted items and splits by tick (`store.rs:1100-1112`).

### Tier 1 — coalesce (DONE)
Merge consecutive delete ops with adjacent targets into one tombstone. No id dropped. Runs on `commit()`. This is `db/README.md` §Actor-local encoding: "Coalesce compatible deletion ranges while preserving deletion event identity."

### Tier 2 — drop content, keep skeleton
`Content::String/Binary` → `Content::Null` on deleted items; keep `id`, `kind`, links, `ticks`.
Condition: delete change ≤ **stable frontier** = per-client `min(ack[c] for c in live_clients)`.

### Tier 3 — physical removal
Frontier condition plus: prune `changes`+`dag` below it (else `doc.rs:224` unwrap panics), relink neighbours, bump **doc epoch**. Client older than epoch gets snapshot-required (`storage/README.md` §Compaction and retention). This is the "retained-history floor".

### Prerequisites for tier 2/3
1. Server-side `ack[client] = ClientState` per doc, persisted. cbdocs calls these "client leases" (`db/README.md` §Hot primary DB). Not built.
2. Liveness: silent > N days → evicted, forced resync. cbdocs proposes 14-day retention for 7-day lease.
3. `pending` empty for that client column.
4. Fix #10 first.

## Tier 1 implementation

Files: `crates/carbyne/src/delete.rs`, `store.rs`, `tx.rs`, `sync.rs` (test).

- `DeleteItem` gains `ticks: u32`; `end_clock() = id.clock + ticks - 1`; `join()` folds next consecutive delete op when target ranges touch.
- Wire: encodes `IdRange(id.client, id.clock, end_clock)` + target range. Same bytes for `ticks == 1`. Not backward compatible; nothing persists yet.
- `DeleteItemStore::coalesce(range)` called at end of `DocStore::commit()`.
- `ItemStore<DeleteItem>::diff` tests `end_clock() > clock`. This is `storage/README.md` §Operation identity: "Select deletes by deletion operation clock… Range selection must use operation spans."
- Prerequisite fix in `tx.rs`: `prepare()` shadowing; `apply()` now marks target range deleted and records the tombstone.

Test: `sync::test::test_commit_coalesces_adjacent_tombstones`. `cargo test -p carbyne`: 109 passed.

Ceiling: within one commit, consecutive ops, touching targets. Cross-commit needs change-id rewriting; defer past tier 2.

## #1 `dirty_blocks` implementation (DONE)

File: `crates/carbyne/src/doc.rs` `Doc::dirty_blocks`.

- For each change with `end > state[client]`: fetch its items (`store.items.get_by_range`) and its deletes (`store.deletes.get_by_range` → `store.find(target)`), walk `parent()` up to the first list whose parent is `None` or the doc root, take `index_of` in that list. Sort + dedup.
- Cost O(changed items × depth). Old code was O(root size) and wrong (returned every block).
- Covers inserts, nested edits (map field, text inside map), deletes, appends. Positions are in the list's full child space (same as `NList::get`), so a deleted block still has a stable position for `BlockStore::sync` to update.

Test: `doc::test::test_dirty_blocks_only_changed` (200 blocks; edit #150, deep-edit #7, delete #42, append → `[7, 42, 150, 200]`; caught-up state → `[]`).

Measured (release, 10k blocks, 5 edited): `dirty_blocks` 52 µs, no-op 0.7 µs. The README's "7 ms" was the criterion loop building 10k blocks, not the call.

Docs updated: `README.md`, `crates/carbyne/README.md`, `crates/carbyne-blocks/README.md`.

Ceiling: `IBTree::index_of` is `range(..).count()` (`index/ibtree.rs:33`), O(position) per lookup. Fine at 10k; perf P1 rank/select index fixes it if lists get very large.

## Next steps, in cost order

Reconciled with `perf/README.md` §Recommended work order and `db/README.md` §Initial implementation policy.

1. ~~**#1** `dirty_blocks`~~ done.
2. **#8 + #7** WAL pragma + upsert in carbyne-blocks. Config-level.
3. **#2** carbyne-db `apply()`: append-only (key per `(db_id, actor, clock_start)`, no read-modify-write), snapshot on threshold not per op. Switch lamport → `ClientState` (#12). This is the first real step of `db/README.md` "Store retained operations as actor-local immutable blocks."
4. **#9 + #10** Fix `Diff` round-trip panic and `Types`/`Embed` encoding. Then varints. Until this, there is no persistable format.
5. **#11** Range-select in `ItemStore::diff` / `ChangeStore::diff` (perf P0). Use `IdRangeMap` (`persist.rs:29`) as the index.
6. **#4** Persist `DocStoreData` snapshot + op tail; hot-doc cache with eviction. Unlocks the 20 GiB → 200 MiB RAM claim and GC tier 2/3.
7. **#5** Delete the fake archive or implement it against the sealed-block design.

Do not start block sealing, compression tuning, or object-storage tiering before steps 3–5. `db/README.md` §Avoid premature work says the same.
