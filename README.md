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
