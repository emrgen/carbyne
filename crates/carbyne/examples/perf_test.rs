use carbyne::Doc;
use std::time::Instant;

fn main() {
    println!("\n=== CRDT Optimization Benchmarks ===\n");

    // Benchmark 1: update_count() optimization
    println!("Test 1: Append 1000 items (tests update_count optimization)");
    let start = Instant::now();
    {
        let doc = Doc::default();
        let list = doc.list();
        for i in 0..1000 {
            list.append(doc.string(&i.to_string()));
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.2}ms", elapsed.as_millis());
    println!("  Before opt: Vec::insert() was O(n) per call");
    println!("  After opt: Direct indexing O(1) per call\n");

    // Benchmark 2: ClientMap clone optimization
    println!("Test 2: Generate diff with 100 map fields");
    let start = Instant::now();
    {
        let doc = Doc::default();
        let map = doc.map();
        for i in 0..100 {
            map.set(&format!("field_{}", i), doc.string(&i.to_string()));
        }
        doc.commit();
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.2}ms", elapsed.as_millis());
    println!("  Before opt: Full ClientMap clone per diff");
    println!("  After opt: Reference only (no allocation)\n");

    // Benchmark 3: Text operations
    println!("Test 3: Append 100 string items to text");
    let start = Instant::now();
    {
        let doc = Doc::default();
        let text = doc.text();
        for i in 0..100 {
            text.append(doc.string(&i.to_string()));
        }
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.2}ms", elapsed.as_millis());
    println!("  Note: Text cache optimization deferred (requires version tracking)\n");

    // Benchmark 4: Repeated commit/diff cycles
    println!("Test 4: 50 iterations of add field + commit");
    let doc = Doc::default();
    let map = doc.map();

    let start = Instant::now();
    for i in 0..50 {
        map.set(&format!("field_{}", i), doc.string(&format!("value-{}", i)));
        doc.commit();
    }
    let elapsed = start.elapsed();
    println!("  Time: {:.2}ms ({:.2}µs per commit)", elapsed.as_millis(), elapsed.as_micros() as f64 / 50.0);
    println!("  Each commit calls diff(), which now doesn't clone ClientMap\n");

    println!("=== Summary ===");
    println!("✅ Opt 1: update_count() — removed Vec::insert() O(n) shifting");
    println!("✅ Opt 2: diff() — removed unnecessary ClientMap clone");
    println!("⏸️  Opt 3: text_content() cache — deferred (needs store versioning)\n");
}
