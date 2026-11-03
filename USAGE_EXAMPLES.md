# Carbyne Usage Examples

Practical examples using the latest APIs.

## Basic Document Operations

### Creating and Modifying a Document

```rust
use carbyne::doc::Doc;

let doc = Doc::new(Default::default());

// Create a string and add to root map
let hello = doc.string("Hello, World!");
doc.set("greeting", hello);

// Create a list
let list = doc.list();
doc.set("items", list.clone());

// Add items to list
list.append(doc.atom("item1"));
list.append(doc.atom("item2"));
list.append(doc.atom("item3"));

// Commit changes
doc.commit();

// Read values back
assert_eq!(
    doc.get("greeting").unwrap().content().to_json(),
    "Hello, World!"
);
assert_eq!(list.size(), 3);
```

## Text Handling (Efficient Batch Operations)

### Multi-Character Text Inserts (NEW)

Efficient insertion without creating one operation per character:

```rust
use carbyne::doc::Doc;

let doc = Doc::new(Default::default());
let text = doc.text();
doc.set("content", text.clone());

// Old way (creates 5 items for "Hello"):
// text.append(doc.string("H"));
// text.append(doc.string("e"));
// text.append(doc.string("l"));
// text.append(doc.string("l"));
// text.append(doc.string("o"));

// New way (creates 1 item for "Hello"):
doc.append_to_text(&text, "Hello");

assert_eq!(text.text_content(), "Hello");
assert_eq!(text.item_ref().borrow().items().len(), 1); // Only 1 item!

// Other batch operations:
doc.prepend_to_text(&text, "Prefix ");
doc.insert_text_at(&text, 10, " Middle");

assert_eq!(text.text_content(), "Prefix Hello Middle");
```

**Performance**: 50-80% fewer operations for typing-heavy workloads.

## Sync and Versioning (NEW)

### Checkpointing the Document State

```rust
use carbyne::doc::Doc;

let doc = Doc::new(Default::default());

// Capture initial state
let checkpoint = doc.frontiers();

// Make changes
let atom = doc.atom("value1");
doc.set("key1", atom);
doc.commit();

// Get new state
let new_checkpoint = doc.frontiers();

// The frontiers changed
assert_ne!(checkpoint, new_checkpoint);
```

### Efficient Change Detection

Detect if there are new changes without computing full diff:

```rust
use carbyne::doc::Doc;

let doc = Doc::new(Default::default());
let state = doc.version();

// Quick check: are there new changes?
assert!(!doc.has_changes_since(state.clone()));

// Make a change
let atom = doc.atom("data");
doc.set("key", atom);

// Now there are changes
assert!(doc.has_changes_since(state));

// Use this in polling loops:
// if doc.has_changes_since(last_state) {
//     let diff = doc.diff(last_state);
//     // Send diff to peers
// }
```

**Performance**: O(1) detection on first change found (avoids full diff construction).

## Advanced: Time-Travel (Foundation)

### API Contract for Future Time-Travel

```rust
use carbyne::doc::Doc;

let doc = Doc::new(Default::default());

// Save a checkpoint
let checkpoint1 = doc.frontiers();

// Make changes
let atom1 = doc.atom("state1");
doc.set("key", atom1.clone());
doc.commit();

let checkpoint2 = doc.frontiers();

// Make more changes
let atom2 = doc.atom("state2");
doc.set("key", atom2.clone());
doc.commit();

// API established but not yet implemented
let result = doc.checkout(checkpoint2);

// Currently returns error (prevents partial implementation)
// Future: will restore document to checkpoint2 state
assert!(result.is_err());
assert!(result.unwrap_err().contains("not yet implemented"));

// Once implemented:
// doc.checkout(checkpoint1)?;  // Restore to first checkpoint
// assert_eq!(doc.get("key"), None);  // key was not set yet
```

**Implementation status**: API contract established, state rebuild algorithm deferred.

## Performance Tips

### 1. Batch Text Operations

Instead of:
```rust
for ch in text.chars() {
    text_node.append(doc.string(ch.to_string()));
}
```

Use:
```rust
doc.append_to_text(&text_node, text);  // Single operation
```

**Result**: 5-10x fewer operations.

### 2. Check Before Diff

Instead of:
```rust
let diff = doc.diff(old_state);
if diff.is_empty() {
    // Skip sync
}
```

Use:
```rust
if !doc.has_changes_since(old_state) {
    // Skip sync immediately
}
let diff = doc.diff(old_state);  // Only if needed
```

**Result**: Faster polling loops, less CPU in heartbeats.

### 3. Save Checkpoints Strategically

```rust
let milestones = vec![];

// Save checkpoint at strategic points
milestones.push(doc.frontiers());

// Do batch of work
for item in items {
    doc.set(&format!("item_{}", item.id), doc.atom(item.data));
}
doc.commit();

milestones.push(doc.frontiers());

// Can restore to any milestone (when time-travel is implemented)
// doc.checkout(milestones[0])?;  // Back to before batch
```

## Common Patterns

### Map with Multiple Entries

```rust
let doc = Doc::new(Default::default());

let data = doc.map();
doc.set("data", data.clone());

data.set("name", doc.string("Alice"));
data.set("age", doc.atom(30));
data.set("email", doc.string("alice@example.com"));

doc.commit();

assert_eq!(data.get("name").unwrap().content().to_json(), "Alice");
assert_eq!(data.get("age").unwrap().content().to_json(), 30);
```

### Nested Structures

```rust
let doc = Doc::new(Default::default());

let root = doc.map();
doc.set("root", root.clone());

let users = doc.list();
root.set("users", users.clone());

let user1 = doc.map();
user1.set("id", doc.atom(1));
user1.set("name", doc.string("Alice"));
users.append(user1);

let user2 = doc.map();
user2.set("id", doc.atom(2));
user2.set("name", doc.string("Bob"));
users.append(user2);

doc.commit();

assert_eq!(users.size(), 2);
```

### Rich Text with Marks (Formatting)

```rust
let doc = Doc::new(Default::default());
let text = doc.text();
doc.set("content", text.clone());

// Add text with formatting
let s1 = doc.string("Hello");
text.append(s1.clone());

// Add marks (future: implement mark API fully)
// s1.add_mark(Mark::bold());

doc.commit();
```

## Testing

### Unit Test Example

```rust
#[test]
fn test_document_operations() {
    let doc = Doc::new(Default::default());
    
    let item = doc.atom("test");
    doc.set("key", item);
    
    assert_eq!(doc.get("key").unwrap().content().to_json(), "test");
    
    let checkpoint = doc.frontiers();
    doc.set("key", doc.atom("updated"));
    
    // Future: time-travel
    // doc.checkout(checkpoint)?;
    // assert_eq!(doc.get("key").unwrap().content().to_json(), "test");
}
```

## See Also

- [`crates/carbyne/README.md`](crates/carbyne/README.md) — Project overview
- [`cbdocs/concepts/`](../cbdocs/concepts/) — Core CRDT concepts (frontier, DAG, time-travel)
- Tests in `src/doc.rs` — More examples in test suite

