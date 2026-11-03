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
| 2  | carbyne-db `apply()`: read whole log, rewrite, then snapshot+delete per op | `changelog.rs:34-43,57-62`, `instance.rs:76-90` (old)         | 3 durable writes/op, log read ∝ batch size. **DONE**, see below.                                                 |
| 3  | No tombstone GC                                                            | `item.rs:178-184`, `store.rs:686-698`                         | Monotonic growth. Tier 1 done.                                                                                   |
| 4  | No CRDT persistence anywhere                                               | `persist.rs:41-45`, `diffstore.rs:35-39`, `changelog.rs:8,47` | All docs resident. RAM is the bill (`db/cost` §Resident memory: 10k docs × 2 MiB = 20 GiB vs 200 MiB hot cache). |
| 5  | Archive pipeline is a stub                                                 | `archive_job.rs:153-164`, `s3_backend.rs:69-76`               | No cold tier.                                                                                                    |
| 6  | `blocks.version` = bincode `ClientState` per row                           | `carbyne-blocks/src/lib.rs:230`                               | 8·C bytes × N rows.                                                                                              |
| 7  | Per-row `SELECT 1` then INSERT/UPDATE                                      | `lib.rs:98-111`                                               | 2× statements.                                                                                                   |
| 8  | No `journal_mode=WAL` / `synchronous=NORMAL`                               | `lib.rs:60-71`, `storage.rs:29-51`                            | fsync per txn.                                                                                                   |
| 9  | Fixed-width uncompressed codec; `Diff` round-trip panics                   | `codec_v1.rs:64-90,155` (old)                                 | 3–5× egress. Round-trip + panic **DONE**, see below. Varints not done.                                           |
| 10 | `Content::Types`/`Embed` encode nothing                                    | `item.rs:1337-1342` (old)                                     | Silent data loss. **DONE**: `Embed` + full `Any` codec; `Types` is derived-only, now `unreachable!`/`Err`.       |
| 11 | `diff()` scans all items + changes per client                              | `store.rs:1100-1116`, `change.rs:458-475` (old)               | O(history) per sync. **DONE**, see below. Also fixed: straddling string shipped wrong suffix.                    |
| 12 | carbyne-db lamport `u64` instead of VV                                     | `instance.rs`, `message.rs` (old)                             | **DONE** with #2: `DbInstance::version() -> ClientState`, wire carries `ClientState::to_bytes()`.                |

Pre-existing, found while implementing, not fixed:
- ~~`Diff` wire round-trip panics~~ fixed under #9.
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

## #9 `Diff` codec round-trip (DONE)

Files: `crates/carbyne/src/codec_v1.rs`, `change.rs`, `diff.rs`, `state.rs`.

- `encode_item`/`decode_item` disagreed: decoder read `target`/`mover` that encoder never wrote, `parent_id` was unconditional on one side and flag-gated on the other, `side` read 8 bits for a 2-bit field. Now symmetric; `parent_id` gated by flag bit 6, `side` masked `& 0b11`.
- `ChangeStore::encode` wrote the total change count then encoded only committed ones → decoder read past the buffer. Now writes `map.len()`.
- `DecoderV1` panicked on short/garbage input (`out of bounds`, `invalid version`). Now `try_new` + bounds-checked `take()` return `Err`; `Diff::from_bytes`/`ClientState::from_bytes` use it. Network bytes can no longer crash the process.
- Added `Diff::to_bytes/from_bytes`, `ClientState::to_bytes/from_bytes`.

Tests: `diff::test::test_encode_decode_diff` (now covers committed list+delete), `carbyne-db/tests/integration_sync.rs::test_malformed_payload_is_error_not_panic`.

## #2 carbyne-db append-only log + snapshot (DONE)

Files: `crates/carbyne-db/src/changelog.rs`, `instance.rs`, `registry.rs`, `sync/{message,daemon,mod}.rs`.

Keys per db: `tail:{db}:{seq:020}` = one `Diff` per commit; `snap:{db}` = full `Diff` (`doc.diff(ClientState::default())`); `snap:{db}:seq`.

- `DbInstance::commit()`: `doc.commit()`, diff since last persisted version, one `put`. Empty diff → no write. `apply(&Diff)`: put then mutate (durability before visibility).
- Every `SNAPSHOT_EVERY = 64` appends: write full snapshot, delete tail ≤ seq. `load()` = `Doc::from(snapshot)` + replay tail. Genesis snapshot written on create so replay rebuilds the same root/client identity (a `Doc::default()` has a fresh id; applying tail diffs to it silently drops everything).
- `DbRegistry::create_from(name, &full_diff)`: bootstrap a peer replica with the same doc identity, fresh client id. Required before incremental sync.
- Lamport removed (#12). `version() -> ClientState`, `diff_since(ClientState)`, `seq()` is tail position only.
- Wire (`DbMessage`): `version: Vec<u8>`, `SyncRequest{since}`, `SyncResponse{diff}`, `OpsBroadcast{diff}`, all carbyne v1 codec bytes. Daemon: request → `diff_since` → response; response/broadcast → `apply` → fan-out to other peers. Malformed payload → `Err`, loop continues.

Tests: `carbyne-db/tests/integration_durability.rs` (`test_snapshot_compacts_tail_and_restart_matches`: 70 commits → tail_len 6, restart equal; `test_apply_remote_diff_persists_and_syncs`), `integration_sync.rs::test_sync_request_response_round_trip`.

Ceiling: snapshot is a full `diff()` (#11 cost); `tail_len()` iterates keys; no ack tracking so tail can only be compacted by snapshot, not by peer coverage. Those are #11 and GC tier 2 prerequisites.

## #11 range-select `diff` (DONE)

Files: `crates/carbyne/src/store.rs` (`newer_than`, `diff_split`, four `IdClockDiff` impls), `change.rs` `ChangeStore::diff`.

- Items: `BTreeMap<Id,_>` is ordered by `(client, clock)`; per-client stores hold one client, so `range(Id(client, clock+1)..)` plus one `next_back()` for the straddling item is O(log n + k).
- Changes: `ChangeId::Ord` treats overlapping ranges as `Equal`, so `set.range(ChangeId(tick+1, tick+1)..)` starts at the first change ending after `tick`.
- Found while writing the boundary test: old straddle logic called `data.split(clock)` with an **absolute** clock where `split` takes an item-relative offset. A peer that had seen `"abc"` of `"abcdef"` received `"f"`, not `"def"`. Now `split(clock + 1 - id.clock)`.

Measured (release, 20k atoms / 200 changes, diff of 1 new item ×100): 45 ms → 7.7 ms. Full diff unchanged (~2 ms).

Tests: `store::tests::test_item_store_diff_boundaries` (before/at/straddle/after clock), `sync::test::test_sync_partial_string_straddle` (fails on old code with `"f"` ≠ `"def"`).

Ceiling: remaining ~77 µs/diff is `ClientStore` iteration over every client plus `Diff::optimize`; fine until client counts are in the thousands.

## Next steps, in cost order

Reconciled with `perf/README.md` §Recommended work order and `db/README.md` §Initial implementation policy.

1. ~~**#1** `dirty_blocks`~~ done.
2. **#8 + #7** WAL pragma + upsert in carbyne-blocks. Config-level.
3. ~~**#2** carbyne-db append-only + `ClientState` (#12)~~ done.
4. ~~**#9** `Diff` round-trip~~ done. ~~**#10** `Types`/`Embed` encoding~~ done (`item.rs` `Any` encode/decode, all 17 shapes; `diff::test::test_encode_decode_diff` covers an `Any::Map` embed). Varints still open.
5. ~~**#11** Range-select diff~~ done.
6. **#4** Persist `DocStoreData` snapshot + op tail; hot-doc cache with eviction. Unlocks the 20 GiB → 200 MiB RAM claim and GC tier 2/3.
7. **#5** Delete the fake archive or implement it against the sealed-block design.

Do not start block sealing, compression tuning, or object-storage tiering before steps 3–5. `db/README.md` §Avoid premature work says the same.
