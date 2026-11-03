use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Peer configuration for a single peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerConfig {
    /// Unique peer ID (e.g., "node-1")
    pub id: String,
    /// Host/IP address (e.g., "localhost", "192.168.1.10")
    pub host: String,
    /// Port to connect to (e.g., 9001)
    pub port: u16,
}

/// Database replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbReplicationConfig {
    /// Local peer ID
    pub local_peer_id: String,
    /// List of all peers in the cluster
    pub peers: Vec<PeerConfig>,
    /// Auto-reconnect interval in seconds (0 = disabled)
    pub auto_reconnect_interval_secs: u64,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
}

impl DbReplicationConfig {
    /// Load config from TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| crate::error::DbError::ConfigError(e.to_string()))?;
        let config = toml::from_str(&content)
            .map_err(|e| crate::error::DbError::ConfigError(e.to_string()))?;
        Ok(config)
    }

    /// Save config to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::DbError::ConfigError(e.to_string()))?;
        fs::write(path, content)
            .map_err(|e| crate::error::DbError::ConfigError(e.to_string()))?;
        Ok(())
    }

    /// Get a peer by ID
    pub fn get_peer(&self, peer_id: &str) -> Option<&PeerConfig> {
        self.peers.iter().find(|p| p.id == peer_id)
    }

    /// Get all remote peers (excluding local)
    pub fn remote_peers(&self) -> Vec<&PeerConfig> {
        self.peers
            .iter()
            .filter(|p| p.id != self.local_peer_id)
            .collect()
    }

    /// Get peer count (including local)
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get remote peer count
    pub fn remote_peer_count(&self) -> usize {
        self.remote_peers().len()
    }

    /// Validate config
    pub fn validate(&self) -> Result<()> {
        if self.local_peer_id.is_empty() {
            return Err(crate::error::DbError::ConfigError(
                "local_peer_id cannot be empty".to_string(),
            ));
        }

        if self.peers.is_empty() {
            return Err(crate::error::DbError::ConfigError(
                "peers list cannot be empty".to_string(),
            ));
        }

        if !self.peers.iter().any(|p| p.id == self.local_peer_id) {
            return Err(crate::error::DbError::ConfigError(
                "local_peer_id not found in peers list".to_string(),
            ));
        }

        for peer in &self.peers {
            if peer.id.is_empty() {
                return Err(crate::error::DbError::ConfigError(
                    "Peer id cannot be empty".to_string(),
                ));
            }
            if peer.host.is_empty() {
                return Err(crate::error::DbError::ConfigError(
                    "Peer host cannot be empty".to_string(),
                ));
            }
            if peer.port == 0 {
                return Err(crate::error::DbError::ConfigError(
                    "Peer port must be > 0".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = DbReplicationConfig {
            local_peer_id: "node-1".to_string(),
            peers: vec![
                PeerConfig {
                    id: "node-1".to_string(),
                    host: "localhost".to_string(),
                    port: 9001,
                },
                PeerConfig {
                    id: "node-2".to_string(),
                    host: "localhost".to_string(),
                    port: 9002,
                },
            ],
            auto_reconnect_interval_secs: 5,
            connection_timeout_secs: 10,
        };

        assert_eq!(config.peer_count(), 2);
        assert_eq!(config.remote_peer_count(), 1);
    }

    #[test]
    fn test_config_validation() {
        let config = DbReplicationConfig {
            local_peer_id: "node-1".to_string(),
            peers: vec![
                PeerConfig {
                    id: "node-1".to_string(),
                    host: "localhost".to_string(),
                    port: 9001,
                },
            ],
            auto_reconnect_interval_secs: 5,
            connection_timeout_secs: 10,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_fails_empty_id() {
        let config = DbReplicationConfig {
            local_peer_id: "".to_string(),
            peers: vec![],
            auto_reconnect_interval_secs: 5,
            connection_timeout_secs: 10,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_fails_missing_local() {
        let config = DbReplicationConfig {
            local_peer_id: "node-1".to_string(),
            peers: vec![
                PeerConfig {
                    id: "node-2".to_string(),
                    host: "localhost".to_string(),
                    port: 9002,
                },
            ],
            auto_reconnect_interval_secs: 5,
            connection_timeout_secs: 10,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_get_peer() {
        let config = DbReplicationConfig {
            local_peer_id: "node-1".to_string(),
            peers: vec![
                PeerConfig {
                    id: "node-1".to_string(),
                    host: "localhost".to_string(),
                    port: 9001,
                },
                PeerConfig {
                    id: "node-2".to_string(),
                    host: "192.168.1.10".to_string(),
                    port: 9002,
                },
            ],
            auto_reconnect_interval_secs: 5,
            connection_timeout_secs: 10,
        };

        let peer = config.get_peer("node-2");
        assert!(peer.is_some());
        assert_eq!(peer.unwrap().host, "192.168.1.10");

        let missing = config.get_peer("node-99");
        assert!(missing.is_none());
    }
}
