/// Complete Pipeline Example
/// 
/// Shows all three phases working together:
/// 1. Application stores session (auto-compressed)
/// 2. Nightly job archives old sessions
/// 3. Query checks both hot and cold storage (optional)

use carbyne_blocks::{SessionStore, ArchiveJob, ArchiveConfig, S3Backend, S3Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Complete Storage Pipeline\n");
    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│ Phase 1: Application stores session (compressed)        │");
    println!("│ Phase 2: Nightly archives old sessions (to S3)          │");
    println!("│ Phase 3: Query retrieves from hot or cold storage       │");
    println!("└─────────────────────────────────────────────────────────┘\n");

    // ========================================================================
    // Setup
    // ========================================================================
    println!("⚙️  SETUP\n");

    let db = rusqlite::Connection::open_in_memory()?;
    let store = SessionStore::new(db)?;
    let s3_config = S3Config::default();
    let s3_backend = S3Backend::new(s3_config)?;

    println!("✓ SessionStore initialized");
    println!("✓ S3Backend configured: {}", s3_backend.stats()?.config.bucket);
    println!();

    // ========================================================================
    // Phase 1: Application Code (Normal Usage)
    // ========================================================================
    println!("📝 PHASE 1: Application stores session (auto-compressed)\n");

    let session_json = r#"{
        "session_id": "sess_live_001",
        "model": "claude-3-sonnet",
        "messages": [
            {"role": "user", "content": "Optimize this database query for 100k rows"},
            {"role": "assistant", "content": "Use indexed WHERE clause and add EXPLAIN PLAN. Here's the optimized version..."}
        ],
        "usage": {"input_tokens": 500, "output_tokens": 245},
        "created_at": 1700000000000
    }"#;

    println!("1. App calls: store_session(session_id, doc_id, json)");
    store.store_session("sess_live_001", "doc_123", session_json)?;
    println!("   ✓ Session stored and auto-compressed\n");

    println!("2. App calls: get_session(session_id)");
    let retrieved = store.get_session("sess_live_001")?;
    println!("   ✓ Session retrieved and auto-decompressed");
    println!("   ✓ Content matches original: {}\n", retrieved == session_json);

    // ========================================================================
    // Phase 2: Nightly Archive Job
    // ========================================================================
    println!("🌙 PHASE 2: Nightly archive job runs at 2 AM\n");

    println!("1. Job detects old sessions (modified >7 days ago)");
    println!("   In real DB: WHERE modified_at < NOW() - INTERVAL 7 DAYS\n");

    println!("2. Job creates archive batch and uploads to S3");
    let archive_config = ArchiveConfig {
        archive_after_days: 7,
        batch_size: 1000,
        dry_run: false,
    };

    let result = ArchiveJob::run(&store, &archive_config)?;
    println!("   ✓ ArchiveJob::run completed");
    println!("   ✓ Config: archive_after_days={}, batch_size={}",
             archive_config.archive_after_days, 
             archive_config.batch_size);

    if result.archived_count > 0 {
        println!("   ✓ Archived {} sessions", result.archived_count);
        println!("   ✓ S3 paths: {:?}", result.s3_paths.iter().take(2).collect::<Vec<_>>());
    } else {
        println!("   ℹ️  No sessions archived yet (none older than 7 days)");
    }
    println!();

    // ========================================================================
    // Storage Inspection
    // ========================================================================
    println!("📊 STORAGE STATUS\n");

    let stats = store.stats()?;
    println!("Hot Storage (SQLite):");
    println!("  • Sessions:           {}", stats.hot_sessions);
    println!("  • Size:               {:.2} MB", stats.hot_size_mb());
    println!("  • Compression ratio:  {:.1}%", stats.compression_ratio * 100.0);
    println!();

    println!("Cold Storage (S3):");
    println!("  • Sessions:           {}", stats.archived_sessions);
    println!("  • Note: Would require S3 query to get exact size");
    println!();

    // ========================================================================
    // Phase 3: Query Hybrid Storage (Optional)
    // ========================================================================
    println!("🔍 PHASE 3: Query hot or cold storage\n");

    println!("Pattern 1: Session exists in hot storage");
    println!("  Code: get_session('sess_live_001')");
    println!("    ├─ Check SQLite");
    println!("    ├─ Found!");
    println!("    └─ Return (decompressed)");
    println!("  Latency: <10ms\n");

    println!("Pattern 2: Session archived (requires Phase 3 implementation)");
    println!("  Code: get_session('sess_archived_001')");
    println!("    ├─ Check SQLite");
    println!("    ├─ Not found");
    println!("    ├─ Check S3");
    println!("    ├─ Download from s3://docs-archive/doc_123/sess_archived_001.zst");
    println!("    ├─ Decompress");
    println!("    ├─ Cache in SQLite");
    println!("    └─ Return (decompressed)");
    println!("  Latency: 50-500ms (S3 + decompress)\n");

    // ========================================================================
    // Production Monitoring Example
    // ========================================================================
    println!("📈 PRODUCTION MONITORING\n");

    println!("Metrics to track:");
    println!("  • hot_size_mb: Current = {:.2} MB", stats.hot_size_mb());
    println!("    Alert if > 6000 (110% of baseline 5.5GB)\n");

    println!("  • compression_ratio: Current = {:.1}%", stats.compression_ratio * 100.0);
    println!("    Expected: 30-40% for JSON\n");

    println!("  • archived_sessions: Current = {}", stats.archived_sessions);
    println!("    Should grow by ~1200/day with nightly job\n");

    println!("Dashboard queries:");
    println!("  1. SELECT DATE(created_at), COUNT(*), AVG(LENGTH(content))");
    println!("     FROM sessions GROUP BY DATE(created_at)");
    println!();
    println!("  2. SELECT COUNT(*) FROM sessions");
    println!("     WHERE archived_at IS NOT NULL");
    println!();
    println!("  3. SELECT SUM(LENGTH(content))/1024/1024 as size_mb");
    println!("     FROM sessions WHERE archived_at IS NULL");
    println!();

    // ========================================================================
    // Cost Analysis
    // ========================================================================
    println!("💰 COST ANALYSIS (100k docs)\n");

    let docs_estimate = 100_000;
    let sessions_per_doc = 3;
    let avg_session_bytes = 700;  // After compression

    let hot_docs_pct = 0.2;  // 20% of docs active this week
    let hot_sessions = (docs_estimate as f64 * hot_docs_pct * sessions_per_doc as f64) as u64;
    let hot_bytes = hot_sessions * avg_session_bytes as u64;
    let hot_cost = (hot_bytes as f64 / 1024.0 / 1024.0 / 1024.0) * 24.0;  // $24/TB-month

    let archived_docs_pct = 0.8;
    let archived_sessions = (docs_estimate as f64 * archived_docs_pct * sessions_per_doc as f64) as u64;
    let archived_bytes = archived_sessions * avg_session_bytes as u64;
    let archived_cost = (archived_bytes as f64 / 1024.0 / 1024.0 / 1024.0) * 0.023;  // S3: $0.023/GB

    println!("Scenario: 100k docs, 3 sessions/doc, 20% active/week");
    println!();
    println!("Hot storage (SQLite):");
    println!("  Sessions:  {:>6}", hot_sessions);
    println!("  Size:      {:>6.1} GB", hot_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("  Cost:      ${:>6.0}/month", hot_cost);
    println!();
    println!("Archived (S3):");
    println!("  Sessions:  {:>6}", archived_sessions);
    println!("  Size:      {:>6.1} GB", archived_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("  Cost:      ${:>6.0}/month", archived_cost);
    println!();
    println!("Total:     ${:>6.0}/month", hot_cost + archived_cost);
    println!();

    // ========================================================================
    // Deployment Checklist
    // ========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("✅ DEPLOYMENT CHECKLIST\n");

    println!("✓ Phase 1 - Compression (Week 1)");
    println!("  ✓ Add zstd dependency");
    println!("  ✓ Verify integration test");
    println!("  ✓ Deploy release binary");
    println!();

    println!("Phase 2 - Archive (Week 2)");
    println!("  [ ] Create S3 bucket");
    println!("  [ ] Setup cron job (2 AM daily)");
    println!("  [ ] Verify nightly archive runs");
    println!("  [ ] Monitor S3 upload success");
    println!();

    println!("Phase 3 - S3 Fallback (Week 3, optional)");
    println!("  [ ] Implement BlockStore::get_session() fallback");
    println!("  [ ] Add AWS credentials to environment");
    println!("  [ ] Test retrieval from S3");
    println!("  [ ] Monitor cold hit rate");
    println!();

    println!("📚 Documentation");
    println!("  • COMPRESSION.md — Architecture and tradeoffs");
    println!("  • DEPLOYMENT.md — Step-by-step production guide");
    println!("  • crates/carbyne-blocks/README.md — API reference");
    println!();

    println!("✅ Pipeline complete!");

    Ok(())
}
