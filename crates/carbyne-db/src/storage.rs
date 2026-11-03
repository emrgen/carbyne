use crate::error::{DbError, Result};
use rocksdb::{DB, Options};
use std::sync::Arc;

pub struct Storage {
    db: Arc<DB>,
}

impl Storage {
    pub fn new(path: &str) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path)
            .map_err(|e| DbError::StorageError(e.to_string()))?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db
            .put(key.as_bytes(), value)
            .map_err(|e| DbError::StorageError(e.to_string()))
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.db
            .get(key.as_bytes())
            .map_err(|e| DbError::StorageError(e.to_string()))
    }

    pub fn delete(&self, key: &str) -> Result<()> {
        self.db
            .delete(key.as_bytes())
            .map_err(|e| DbError::StorageError(e.to_string()))
    }
}

impl Clone for Storage {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}
