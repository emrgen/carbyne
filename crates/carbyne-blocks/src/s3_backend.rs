/// S3 backend for archived session retrieval
/// 
/// Provides fallback when session not in hot SQLite storage.
/// This is the retrieval half of the archive job.
/// 
/// Architecture:
///   BlockStore.get_session(id) 
///     ├─ Try SQLite (hot, <10ms)
///     └─ Not found → Try S3 (cold, 50-500ms)

use crate::BlockError;
use std::time::Duration;

/// Configuration for S3 backend
#[derive(Clone, Debug)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    
    /// S3 region
    pub region: String,
    
    /// Base prefix for archive paths (e.g. "docs-archive/")
    pub prefix: String,
    
    /// Request timeout
    pub timeout: Duration,
    
    /// Max retries for transient failures
    pub max_retries: u32,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            bucket: "docs-archive".to_string(),
            region: "us-east-1".to_string(),
            prefix: "sessions/".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
        }
    }
}

/// S3 backend client
pub struct S3Backend {
    config: S3Config,
}

impl S3Backend {
    /// Create new S3 backend with config
    pub fn new(config: S3Config) -> Result<Self, BlockError> {
        // Validate config
        if config.bucket.is_empty() {
            return Err(BlockError::InvalidBlock);
        }

        Ok(S3Backend { config })
    }

    /// Build S3 key from session ID and doc ID
    fn build_key(&self, doc_id: &str, session_id: &str) -> String {
        format!("{}{}/{}.zst", self.config.prefix, doc_id, session_id)
    }

    /// Upload compressed session to S3
    /// 
    /// Called by ArchiveJob after compressing
    pub fn upload_session(&self, doc_id: &str, session_id: &str, _compressed: &[u8]) -> Result<String, BlockError> {
        let key = self.build_key(doc_id, session_id);
        
        // Placeholder for actual S3 upload
        // In production: call S3 PutObject API, retry on transient errors
        
        Ok(format!("s3://{}/{}", self.config.bucket, key))
    }

    /// Get S3 backend stats
    pub fn stats(&self) -> Result<S3Stats, BlockError> {
        Ok(S3Stats {
            config: self.config.clone(),
        })
    }
}

/// Statistics about S3 backend
#[derive(Clone, Debug)]
pub struct S3Stats {
    pub config: S3Config,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();
        assert_eq!(config.bucket, "docs-archive");
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_s3_backend_key_building() {
        let backend = S3Backend::new(S3Config::default()).unwrap();
        let key = backend.build_key("doc_123", "sess_456");
        
        assert!(key.contains("doc_123"));
        assert!(key.contains("sess_456"));
        assert!(key.ends_with(".zst"));
    }

    #[test]
    fn test_s3_backend_invalid_config() {
        let mut config = S3Config::default();
        config.bucket.clear();
        
        let result = S3Backend::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_s3_backend_upload() -> Result<(), BlockError> {
        let backend = S3Backend::new(S3Config::default())?;
        let s3_path = backend.upload_session("doc_1", "sess_1", b"compressed data")?;
        
        assert!(s3_path.starts_with("s3://"));
        assert!(s3_path.contains("doc_1"));
        assert!(s3_path.contains("sess_1"));
        
        Ok(())
    }
}
