# Carbyne – CRDT Monorepo

High-performance CRDT library with pluggable algorithms.

**Status**: Under development. Not for production.

## Workspace Structure

```
carbyne/
├── Cargo.toml           # Workspace root
├── crates/
│   └── carbyne/         # Main CRDT library
│       ├── src/
│       ├── examples/
│       └── Cargo.toml
└── README.md
```

## Building

Requires Rust nightly (for `slice_take` feature).

```bash
cd carbyne
rustup override set nightly
cargo build --release
```

## Modules

- **Core**: `doc`, `store`, `change` – document, item storage, change tracking
- **Data Types**: `nmap`, `nlist`, `ntext`, `nstring`, `natom` – CRDT containers
- **Algorithms**: `crdt_yata`, `crdt_fugue` – conflict resolution strategies
- **Sync**: `sync`, `diff`, `codec_v1` – synchronization and serialization
- **Indexing**: `index/` – B-tree, red-black tree, skip list
- **Testing**: `examples/` – integration tests

## Experiments

Add new crates to `crates/` as needed:
- `crates/carbyne-algo` – swappable algorithm layer (future)
- `crates/carbyne-bindings` – language bindings (future)

Update `Cargo.toml` members list and run `cargo build -w`.

## Features

- [x] Document model (map, list, text, atom)
- [x] YATA algorithm
- [x] Sync protocol (basic)
- [x] Diff and merge
- [ ] Undo/redo manager
- [ ] Movable list/tree
- [ ] Counter CRDT
- [ ] Fugue algorithm
- [ ] Time travel / version control

## Move Cycle Handling

Status: **Partially implemented**. Cycle detection works; full redo logic deferred.

+ ✅ `creates_cycle(parent, child)` – checks if move creates tree cycle
+ ✅ `has_mover` flag on changes – marks which contain move operations  
+ ✅ Movers stored separately – quick lookup for undo/redo
+ ⚠️  **Gap**: Per move-op.pdf, when cycle detected should undo dependent moves & reapply
   - Current: just skips the cycle-causing move (see doc.rs:246)
   - Missing: logic to find dependent moves, undo, reapply in topological order

**To expose move-skip detection API:**
- Need: `HashMap<ChangeId, Vec<ChangeId>>` reverse dependency graph
- Time: 1–2h to add + maintain in DAG.insert()
- Then: expose `doc.has_mover(change_id)` and `doc.dependents(change_id)`

**Optimization summary:**
- Core DAG: BTreeSet, HashMap – already optimized ✅
- `change_sorter.update_count()`: O(n) per insert – TODO marked, not blocking
- Reverse deps: currently O(n) scan – will add HashMap for O(1) lookup later

See: `crates/carbyne/src/cycle.rs`, `nmove.rs`, `dag.rs`, `change.rs:ChangeNodeFlags::MOVE`
