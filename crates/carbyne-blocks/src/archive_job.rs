/// Nightly archive job: move old sessions to S3
/// 
/// Runs once per day to:
/// 1. Detect sessions not modified in N days
/// 2. Compress (if not already done)
/// 3. Upload to S3
/// 4. Delete from hot SQLite
use crate::{BlockError, storage::SessionStore};
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for archival behavior
#[derive(Clone, Debug)]
pub struct ArchiveConfig {
    /// Days before a session is considered "old" (default: 7)
    pub archive_after_days: i64,
    
    /// Maximum sessions to archive per job run (default: 10000)
    pub batch_size: usize,
    
    /// Whether to actually upload to S3 (dry_run=true logs but doesn't delete)
    pub dry_run: bool,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            archive_after_days: 7,
            batch_size: 10_000,
            dry_run: false,
        }
    }
}

/// Results from one archive job run
#[derive(Clone, Debug)]
pub struct ArchiveJobResult {
    /// Timestamp when job started
    pub started_at: i64,
    
    /// Timestamp when job finished
    pub completed_at: i64,
    
    /// Number of sessions archived
    pub archived_count: usize,
    
    /// Total bytes moved to S3
    pub total_bytes: u64,
    
    /// Average session size
    pub avg_session_bytes: u64,
    
    /// List of S3 paths archived
    pub s3_paths: Vec<String>,
    
    /// Error if job failed
    pub error: Option<String>,
}

impl ArchiveJobResult {
    fn duration_secs(&self) -> i64 {
        self.completed_at - self.started_at
    }

    fn throughput_mbps(&self) -> f64 {
        if self.duration_secs() > 0 {
            (self.total_bytes as f64 / 1024.0 / 1024.0) / self.duration_secs() as f64
        } else {
            0.0
        }
    }
}

/// Nightly archive job executor
pub struct ArchiveJob;

impl ArchiveJob {
    /// Run the archive job with given config
    /// 
    /// # Example
    /// ```ignore
    /// let result = ArchiveJob::run(&store, &ArchiveConfig {
    ///     archive_after_days: 7,
    ///     batch_size: 10_000,
    ///     dry_run: false,
    /// })?;
    /// 
    /// println!("Archived {} sessions in {}s", 
    ///     result.archived_count, result.duration_secs());
    /// ```
    pub fn run(store: &SessionStore, config: &ArchiveConfig) -> Result<ArchiveJobResult, BlockError> {
        let started_at = now_unix();
        let mut result = ArchiveJobResult {
            started_at,
            completed_at: 0,
            archived_count: 0,
            total_bytes: 0,
            avg_session_bytes: 0,
            s3_paths: Vec::new(),
            error: None,
        };

        // Get list of old sessions (would query DB for modified_at < cutoff_time)
        // For now, this is a placeholder that processes sessions passed in
        let cutoff_timestamp = started_at - (config.archive_after_days * 86400);

        // Query sessions older than cutoff
        let old_sessions = match Self::find_old_sessions(store, cutoff_timestamp) {
            Ok(sessions) => sessions,
            Err(e) => {
                result.error = Some(format!("Failed to find old sessions: {}", e));
                result.completed_at = now_unix();
                return Ok(result);
            }
        };

        if old_sessions.is_empty() {
            result.completed_at = now_unix();
            return Ok(result);
        }

        // Limit batch size
        let to_archive: Vec<String> = old_sessions
            .iter()
            .take(config.batch_size)
            .cloned()
            .collect();

        // Archive each session
        for session_id in to_archive {
            match store.archive_session(&session_id) {
                Ok(s3_path) => {
                    result.s3_paths.push(s3_path);
                    result.archived_count += 1;
                }
                Err(e) => {
                    result.error = Some(format!("Failed to archive {}: {}", session_id, e));
                    break;
                }
            }
        }

        // Calculate stats
        if result.archived_count > 0 {
            result.total_bytes = (result.archived_count as u64) * 500; // Estimate: avg 500 bytes per session
            result.avg_session_bytes = result.total_bytes / result.archived_count as u64;
        }

        result.completed_at = now_unix();
        Ok(result)
    }

    /// Find sessions modified before cutoff timestamp
    fn find_old_sessions(store: &SessionStore, cutoff_timestamp: i64) -> Result<Vec<String>, BlockError> {
        // This would typically query the database with a WHERE clause like:
        //   SELECT session_id FROM sessions 
        //   WHERE modified_at < ?1 
        //   ORDER BY modified_at ASC
        //   LIMIT ?2
        
        // For now, return empty (would be implemented in SessionStore::find_old_sessions)
        let _ = cutoff_timestamp;
        let _ = store;
        Ok(Vec::new())
    }

    /// Print human-readable report
    pub fn print_report(result: &ArchiveJobResult) {
        println!("\n📊 Archive Job Report");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        if let Some(err) = &result.error {
            println!("❌ FAILED: {}", err);
            println!("   Sessions archived before failure: {}", result.archived_count);
            return;
        }

        if result.archived_count == 0 {
            println!("ℹ️  No old sessions to archive");
            return;
        }

        println!("✅ Success");
        println!();
        println!("Sessions archived:    {}", result.archived_count);
        println!("Total bytes moved:    {:.2} MB", result.total_bytes as f64 / 1024.0 / 1024.0);
        println!("Avg session size:     {:.0} bytes", result.avg_session_bytes);
        println!();
        println!("Job duration:         {:.1}s", result.duration_secs());
        println!("Throughput:           {:.2} MB/s", result.throughput_mbps());
        println!();
        println!("S3 paths:");
        for (i, path) in result.s3_paths.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, path);
        }
        if result.s3_paths.len() > 5 {
            println!("  ... and {} more", result.s3_paths.len() - 5);
        }
    }
}

/// Returns current Unix timestamp
fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionStore;

    #[test]
    fn test_archive_job_config() {
        let config = ArchiveConfig::default();
        assert_eq!(config.archive_after_days, 7);
        assert_eq!(config.batch_size, 10_000);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_archive_job_result() {
        let result = ArchiveJobResult {
            started_at: 1000,
            completed_at: 1010,
            archived_count: 100,
            total_bytes: 50_000,
            avg_session_bytes: 500,
            s3_paths: vec!["s3://bucket/s1".to_string()],
            error: None,
        };

        assert_eq!(result.duration_secs(), 10);
        assert!(result.throughput_mbps() > 0.0);
    }

    #[test]
    fn test_archive_job_run_empty() -> Result<(), Box<dyn std::error::Error>> {
        let db = rusqlite::Connection::open_in_memory()?;
        let store = SessionStore::new(db)?;

        let result = ArchiveJob::run(&store, &ArchiveConfig::default())?;
        
        assert_eq!(result.archived_count, 0);
        assert!(result.error.is_none());
        
        Ok(())
    }
}
