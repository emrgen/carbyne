use crate::changelog::ChangeLog;
use crate::error::{DbError, Result};
use crate::instance::{DbId, DbInstance, DbName};
use crate::storage::Storage;
use carbyne::Diff;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbMetadata {
    pub id: String,
    pub name: String,
    pub created_ts: u64,
}

pub struct DbRegistry {
    instances: DashMap<DbId, Arc<DbInstance>>,
    storage: Storage,
}

impl DbRegistry {
    pub fn new(storage_path: &str) -> Result<Self> {
        let storage = Storage::new(storage_path)?;
        Ok(Self {
            instances: DashMap::new(),
            storage,
        })
    }
    pub fn load_all(&self) -> Result<()> {
        let metadata_entries = self.storage.iter_keys_with_prefix("metadata:")?;

        for (_, metadata_bytes) in metadata_entries {
            let metadata: DbMetadata = bincode::deserialize(&metadata_bytes)
                .map_err(|e| crate::error::DbError::SerializationError(e.to_string()))?;

            let db_id = DbId(uuid::Uuid::parse_str(&metadata.id)
                .map_err(|e| crate::error::DbError::SerializationError(e.to_string()))?);
            let name = DbName(metadata.name.clone());

            // Load from storage (replay changelog)
            let changelog = Arc::new(ChangeLog::new(db_id, self.storage.clone()));
            let instance = Arc::new(DbInstance::from_storage(db_id, name, changelog)?);

            self.instances.insert(db_id, instance);
        }

        Ok(())
    }
    /// Create a fresh, empty db.
    pub fn create(&self, name: DbName) -> Result<Arc<DbInstance>> {
        let n = name.clone();
        self.register(name, |id, log| DbInstance::new(id, n, log))
    }

    /// Create a db seeded from a peer's full diff (`doc.diff(ClientState::default())`).
    /// Required before incremental sync: diffs only apply to a doc with the same identity.
    pub fn create_from(&self, name: DbName, snapshot: &Diff) -> Result<Arc<DbInstance>> {
        let n = name.clone();
        self.register(name, |id, log| DbInstance::from_snapshot(id, n, log, snapshot))
    }

    fn register(
        &self,
        name: DbName,
        build: impl FnOnce(DbId, Arc<ChangeLog>) -> Result<DbInstance>,
    ) -> Result<Arc<DbInstance>> {
        let id = DbId::new();
        let changelog = Arc::new(ChangeLog::new(id, self.storage.clone()));
        let instance = Arc::new(build(id, changelog)?);

        // Save metadata
        let metadata = DbMetadata {
            id: id.as_str(),
            name: name.0.clone(),
            created_ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let key = format!("metadata:{}", id.as_str());
        let value = bincode::serialize(&metadata)
            .map_err(|e| DbError::SerializationError(e.to_string()))?;
        self.storage.put(&key, &value)?;

        self.instances.insert(id, instance.clone());
        Ok(instance)
    }

    pub fn get(&self, db_id: DbId) -> Result<Arc<DbInstance>> {
        self.instances
            .get(&db_id)
            .map(|entry| Arc::clone(&*entry))
            .ok_or(DbError::InstanceNotFound)
    }

    pub fn list(&self) -> Vec<(DbId, DbName)> {
        self.instances
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().name.clone()))
            .collect()
    }

    pub fn delete(&self, db_id: DbId) -> Result<()> {
        self.instances.remove(&db_id);

        // Clean up metadata
        let key = format!("metadata:{}", db_id.as_str());
        self.storage.delete(&key)?;

        Ok(())
    }
}

impl Clone for DbRegistry {
    fn clone(&self) -> Self {
        Self {
            instances: self.instances.clone(),
            storage: self.storage.clone(),
        }
    }
}
