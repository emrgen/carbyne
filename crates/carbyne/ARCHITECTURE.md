# Container APIs

`Type` represents heterogeneous CRDT items with enum dispatch. Common identity, linkage, visibility, serialization, and item-level move operations remain on `Type`.

Container mutations use concrete handles:

```rust
use carbyne::Doc;

let doc = Doc::default();
let list = doc.list();
doc.set("items", list.clone());
list.append(doc.atom("first"));
let dynamic = doc.get("items").unwrap();
let list = dynamic.as_list().expect("items must be a list");
list.insert(0, doc.atom("before"));
```

`Type::as_list`, `as_map`, and `as_text` return checked, shared typed handles. They return `None` for another kind. The universal `Type::append`, `prepend`, `insert`, `set`, `remove`, and `clear` methods are removed; migrate callers to typed handles. `Type::get` remains a map/list query.

- `NList`: append, prepend, insert, clear; offsets count elements. Handles share the list index.
- `NMap`: set, remove, clear; fields use string keys.
- `NText`: append, prepend, insert, clear; inserted items must be strings. Offsets and size currently count UTF-8 bytes; splitting requires valid string boundaries.
- `RichText.text` is `NText`, not `Type`.

Insertion is not a substitute for moving existing items. Existing `Type::move_to`, `move_before`, and `move_after` preserve move semantics.

Typed APIs remove unsupported container operations from the generic handle; they do not make text input validation or offset handling compile-time checks. Keep map conflict projection, list indexing, string splitting, marks, and move-target behavior distinct. No trait objects or new storage layer are needed.

Internal `item_ref()` accessors borrow `&ItemRef`; clone explicitly only when an owned handle is required. Typed container extraction still clones shared handles, preserving canonical runtime state.

`Type::encode` passes borrowed `ItemData` to the encoder without copying payloads. As with `ItemRef::encode`, the immutable item borrow lasts through the encoder callback; callbacks must not mutably borrow the same item.

Enum predicates use `matches!`; owned `Type`-to-`ItemRef` conversion reuses the common accessor. Size, range, splitting, JSON, and serialization retain per-variant dispatch because their semantics differ.
