/// Integration test: 10k session compression and retrieval
/// 
/// Verifies:
/// - Compression ratio across realistic session sizes
/// - No data corruption (compression round-trip)
/// - Latency overhead (<1ms per session)
/// - Storage efficiency

use carbyne_blocks::storage::SessionStore;
use std::time::Instant;

#[test]
fn test_compress_10k_sessions() {
    // Setup in-memory DB
    let db = rusqlite::Connection::open_in_memory()
        .expect("Failed to create in-memory DB");
    let store = SessionStore::new(db)
        .expect("Failed to create SessionStore");

    // Generate realistic session sizes (500-2000 bytes)
    let session_templates = vec![
        // Small: just a question/answer pair
        r#"{"role":"user","content":"What is X?","tokens":50}"#,
        
        // Medium: multi-turn conversation
        r#"{"messages":[{"role":"user","content":"Explain React hooks in detail"},{"role":"assistant","content":"React hooks are functions that let you...","tokens":250}]}"#,
        
        // Large: with context and history
        r#"{"session_id":"s123","doc_id":"d456","model":"claude-3-sonnet","messages":[{"role":"user","content":"Rewrite the following section with better prose...","context":"[2000 chars of doc content]"},{"role":"assistant","content":"Here's the rewritten version...","tokens":500}],"metadata":{"created_at":1699564800000,"edited_at":1699564900000}}"#,
    ];

    let mut total_uncompressed = 0u64;
    let mut total_compressed = 0u64;
    let mut compress_times = Vec::new();
    let mut decompress_times = Vec::new();

    // Store 10k sessions
    println!("\n📦 Storing 10,000 sessions...");
    for i in 0..10_000 {
        let session_id = format!("sess_{:05}", i);
        let doc_id = format!("doc_{:03}", i / 100); // 100 sessions per doc
        
        // Vary session size for realism
        let template_idx = (i as usize) % session_templates.len();
        let mut session_json = session_templates[template_idx].to_string();
        
        // Add iteration metadata
        session_json.push_str(&format!(r#","iteration":{}"#, i));
        
        // Measure compression time
        let start = Instant::now();
        store
            .store_session(&session_id, &doc_id, &session_json)
            .expect("Failed to store session");
        let compress_time = start.elapsed().as_micros();
        compress_times.push(compress_time);

        total_uncompressed += session_json.len() as u64;

        if (i + 1) % 1000 == 0 {
            println!("  ✓ Stored {} sessions", i + 1);
        }
    }

    // Verify stored size from DB
    let stats = store.stats().expect("Failed to get stats");
    total_compressed = (stats.hot_size_mb() * 1024.0 * 1024.0) as u64;

    println!("\n📊 Compression Statistics:");
    println!("  Uncompressed: {:.2} MB ({} bytes)", 
             total_uncompressed as f64 / 1024.0 / 1024.0,
             total_uncompressed);
    println!("  Compressed:   {:.2} MB ({} bytes)", 
             total_compressed as f64 / 1024.0 / 1024.0,
             total_compressed);
    println!("  Ratio:        {:.1}%", stats.compression_ratio * 100.0);
    println!("  Savings:      {:.2} MB ({:.1}%)", 
             (total_uncompressed - total_compressed) as f64 / 1024.0 / 1024.0,
             (1.0 - stats.compression_ratio as f64) * 100.0);

    // Verify compression ratio (expect 30-40% for JSON)
    assert!(
        stats.compression_ratio < 0.5,
        "Compression ratio {:.1}% too high, expected <50%",
        stats.compression_ratio * 100.0
    );

    // Measure decompression time and verify data
    println!("\n⚡ Verifying decompression (sampling 100 sessions)...");
    for i in (0..10_000).step_by(100) {
        let session_id = format!("sess_{:05}", i);
        let doc_id = format!("doc_{:03}", i / 100);
        
        // Generate expected content
        let template_idx = i % session_templates.len();
        let mut expected = session_templates[template_idx].to_string();
        expected.push_str(&format!(r#","iteration":{}"#, i));

        // Measure decompression time
        let start = Instant::now();
        let retrieved = store
            .get_session(&session_id)
            .expect("Failed to retrieve session");
        let decompress_time = start.elapsed().as_micros();
        decompress_times.push(decompress_time);

        // Verify content integrity
        assert_eq!(
            retrieved, expected,
            "Session {} content mismatch after decompression",
            session_id
        );
    }

    // Latency analysis
    let avg_compress = compress_times.iter().sum::<u128>() / compress_times.len() as u128;
    let avg_decompress = decompress_times.iter().sum::<u128>() / decompress_times.len() as u128;
    let max_compress = *compress_times.iter().max().unwrap();
    let max_decompress = *decompress_times.iter().max().unwrap();

    println!("\n⏱️  Latency Analysis:");
    println!("  Compress:     {:.0} µs avg, {:.0} µs max", avg_compress, max_compress);
    println!("  Decompress:   {:.0} µs avg, {:.0} µs max", avg_decompress, max_decompress);
    println!("  Total/session: {:.0} µs", avg_compress + avg_decompress);

    // Verify latency targets (<1ms per session)
    assert!(
        avg_compress < 1000,
        "Compress latency {:.0}µs exceeds 1000µs target",
        avg_compress
    );
    assert!(
        avg_decompress < 1000,
        "Decompress latency {:.0}µs exceeds 1000µs target",
        avg_decompress
    );

    // Test archive functionality
    println!("\n📦 Testing archive (move 100 old sessions to S3)...");
    for i in 0..100 {
        let session_id = format!("sess_{:05}", i);
        let s3_path = store
            .archive_session(&session_id)
            .expect("Failed to archive session");
        
        // Verify S3 path format
        assert!(
            s3_path.starts_with("s3://docs-archive/"),
            "Invalid S3 path: {}",
            s3_path
        );
    }

    let stats_after_archive = store.stats().expect("Failed to get stats after archive");
    println!("  ✓ Archived 100 sessions");
    println!("  Hot sessions remaining: {}", stats_after_archive.hot_sessions);
    println!("  Archived sessions: {}", stats_after_archive.archived_sessions);

    assert_eq!(
        stats_after_archive.hot_sessions,
        10_000 - 100,
        "Hot sessions not updated after archive"
    );

    println!("\n✅ All compression tests passed!");
    println!("  • 10k sessions compressed and verified");
    println!("  • Compression ratio: {:.1}%", stats.compression_ratio * 100.0);
    println!("  • Latency: {:.0}µs compress, {:.0}µs decompress", 
             avg_compress, avg_decompress);
    println!("  • Archive: 100 sessions moved to S3");
}

#[test]
fn test_compression_round_trip() {
    let db = rusqlite::Connection::open_in_memory()
        .expect("Failed to create in-memory DB");
    let store = SessionStore::new(db)
        .expect("Failed to create SessionStore");

    // Test various session sizes
    let test_cases = vec![
        ("tiny", r#"{"role":"user","content":"Hi"}"#),
        ("small", r#"{"id":"1","session":[{"role":"user","content":"Explain..."},{"role":"assistant","content":"...","tokens":100}]}"#),
        ("medium", r#"{"session_id":"s1","model":"gpt-4","messages":[{"role":"user","content":"Long question with context about the project architecture and how it scales...","context_lines":500},{"role":"assistant","content":"Detailed answer with code examples and explanations...","tokens":500}]}"#),
    ];

    for (name, original) in test_cases {
        store
            .store_session(&format!("test_{}", name), "doc_test", original)
            .expect("Failed to store");

        let retrieved = store
            .get_session(&format!("test_{}", name))
            .expect("Failed to retrieve");

        assert_eq!(
            retrieved, original,
            "Round-trip failed for {} session",
            name
        );
    }
}

#[test]
fn test_archive_stats() {
    let db = rusqlite::Connection::open_in_memory()
        .expect("Failed to create in-memory DB");
    let store = SessionStore::new(db)
        .expect("Failed to create SessionStore");

    // Store 10 sessions
    for i in 0..10 {
        store
            .store_session(&format!("sess_{}", i), "doc_1", "test content")
            .expect("Failed to store");
    }

    let stats1 = store.stats().expect("Failed to get stats");
    assert_eq!(stats1.hot_sessions, 10);
    assert_eq!(stats1.archived_sessions, 0);

    // Archive 5
    for i in 0..5 {
        store
            .archive_session(&format!("sess_{}", i))
            .expect("Failed to archive");
    }

    let stats2 = store.stats().expect("Failed to get stats");
    assert_eq!(stats2.hot_sessions, 5);
    assert_eq!(stats2.archived_sessions, 5);
}
