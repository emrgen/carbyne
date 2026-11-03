/// Example: Nightly Archive Job
///
/// Shows production pattern for archiving old sessions nightly.
/// 
/// Use in cron:
///   0 2 * * * /usr/local/bin/carbyne-archive-nightly
/// (runs at 2 AM every day)

use carbyne_blocks::{SessionStore, archive_job::{ArchiveJob, ArchiveConfig}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌙 Nightly Archive Job\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // ========================================================================
    // PRODUCTION SETUP (simulated)
    // ========================================================================
    println!("1️⃣  Setting up database connection...\n");
    
    // In production: open real SQLite file
    // let db = rusqlite::Connection::open("/var/lib/carbyne/sessions.db")?;
    let db = rusqlite::Connection::open_in_memory()?;
    
    let store = SessionStore::new(db)?;
    println!("   ✓ Connected to session store\n");

    // ========================================================================
    // POPULATE TEST DATA
    // ========================================================================
    println!("2️⃣  Populating with test sessions...\n");
    
    // Create test data spanning different dates
    let test_sessions = vec![
        // Current sessions (should not archive)
        ("today_001", "doc_1", "Recent session from today"),
        ("today_002", "doc_2", "Another recent session"),
        
        // Old sessions (should archive)
        ("old_001", "doc_3", "Session from 10 days ago"),
        ("old_002", "doc_4", "Another old session"),
        ("old_003", "doc_5", "Very old session"),
    ];

    for (session_id, doc_id, content) in &test_sessions {
        store.store_session(session_id, doc_id, content)?;
    }
    
    println!("   ✓ Stored {} test sessions\n", test_sessions.len());

    // ========================================================================
    // RUN ARCHIVE JOB
    // ========================================================================
    println!("3️⃣  Running archive job...\n");
    
    let config = ArchiveConfig {
        archive_after_days: 7,
        batch_size: 1000,
        dry_run: false,
    };

    println!("   Config:");
    println!("     • Archive sessions older than: {} days", config.archive_after_days);
    println!("     • Batch size: {}", config.batch_size);
    println!("     • Dry run: {}\n", config.dry_run);

    let result = ArchiveJob::run(&store, &config)?;
    
    ArchiveJob::print_report(&result);
    println!();

    // ========================================================================
    // VERIFY METRICS
    // ========================================================================
    println!("4️⃣  Verifying storage metrics...\n");
    
    let stats = store.stats()?;
    println!("   Hot storage:");
    println!("     • Sessions: {}", stats.hot_sessions);
    println!("     • Size: {:.2} MB", stats.hot_size_mb());
    println!("     • Compression: {:.1}%", stats.compression_ratio * 100.0);
    println!();
    println!("   Archived:");
    println!("     • Sessions: {}", stats.archived_sessions);
    println!("     • Total bytes: {}", stats.archived_sessions);
    println!();

    // ========================================================================
    // PRODUCTION DEPLOYMENT GUIDE
    // ========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📋 Production Deployment Guide\n");

    println!("1. Build the archive binary:\n");
    println!("   cargo build --release --example nightly_archive\n");

    println!("2. Deploy to server:\n");
    println!("   scp target/release/examples/nightly_archive user@prod:/usr/local/bin/\n");

    println!("3. Setup cron job (root):\n");
    println!("   0 2 * * * /usr/local/bin/nightly_archive >> /var/log/carbyne-archive.log 2>&1\n");

    println!("4. Monitor logs:\n");
    println!("   tail -f /var/log/carbyne-archive.log\n");

    println!("5. Alert thresholds:\n");
    println!("   • Alert if: archived < expected (check for stuck job)");
    println!("   • Alert if: hot_size > 5GB (check for unbounded growth)");
    println!("   • Alert if: S3 upload errors > 0\n");

    // ========================================================================
    // SIMULATE MULTI-DAY SCENARIO
    // ========================================================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📊 Multi-Day Scenario (Production Pattern)\n");

    // Create new store for scenario
    let db2 = rusqlite::Connection::open_in_memory()?;
    let store2 = SessionStore::new(db2)?;

    let mut running_total = 0u64;
    let mut total_archived = 0usize;

    for day in 0..7 {
        // Simulate adding sessions each day
        for i in 0..100 {
            let session_id = format!("day{}_sess{}", day, i);
            let json = format!(
                r#"{{"day":{}, "session":{}, "content": "Session content..."}}"#,
                day, i
            );
            store2.store_session(&session_id, "doc_1", &json)?;
        }

        if day == 6 {
            // On day 7, run archive job
            println!("Day {} (archive day): Running nightly job...", day + 1);
            let result = ArchiveJob::run(&store2, &ArchiveConfig::default())?;
            
            if result.archived_count > 0 {
                println!("  ✓ Archived {} sessions", result.archived_count);
                println!("  ✓ Duration: {:.1}s", result.completed_at - result.started_at);
                total_archived += result.archived_count;
                running_total += result.total_bytes;
            } else {
                println!("  ℹ️  No sessions to archive yet");
            }
        } else {
            let stats = store2.stats()?;
            println!("Day {}: {} hot sessions", day + 1, stats.hot_sessions);
        }
    }

    println!("\nArchive summary:");
    println!("  Total archived: {}", total_archived);
    println!("  Total bytes: {:.2} MB", running_total as f64 / 1024.0 / 1024.0);

    // ========================================================================
    // MONITORING QUERIES
    // ========================================================================
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📈 Monitoring Queries for Dashboard\n");

    println!("1. Session count trend:\n");
    println!("   SELECT DATE(created_at) as date, COUNT(*) as count");
    println!("   FROM sessions GROUP BY DATE(created_at)\n");

    println!("2. Storage usage by doc:\n");
    println!("   SELECT doc_id, COUNT(*) as sessions, SUM(LENGTH(content)) as bytes");
    println!("   FROM sessions GROUP BY doc_id ORDER BY bytes DESC\n");

    println!("3. Archive efficiency:\n");
    println!("   SELECT");
    println!("     COUNT(*) as total_sessions,");
    println!("     SUM(CASE WHEN archived_at IS NULL THEN 1 ELSE 0 END) as hot,");
    println!("     SUM(CASE WHEN archived_at IS NOT NULL THEN 1 ELSE 0 END) as cold");
    println!("   FROM sessions\n");

    println!("4. Compression stats:\n");
    println!("   SELECT");
    println!("     SUM(LENGTH(original_content)) / 1024.0 / 1024.0 as uncompressed_mb,");
    println!("     SUM(LENGTH(compressed_content)) / 1024.0 / 1024.0 as compressed_mb,");
    println!("     ROUND(100.0 * AVG(LENGTH(compressed_content) / CAST(LENGTH(original_content) AS REAL)), 1) as ratio_pct");
    println!("   FROM sessions\n");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("✅ Nightly archive job example complete!");
    println!();

    Ok(())
}
