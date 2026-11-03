# carbyne-db Cluster Setup Guide

## Overview

carbyne-db uses TOML configuration files to define peer connections in a cluster. Each database instance can be configured to synchronize with other instances (peers) using the replication protocol.

## Configuration Structure

A cluster configuration file specifies:

1. **local_peer_id** — Identifier for this node (must match one in the peers list)
2. **peers** — List of all nodes in the cluster with their endpoints
3. **auto_reconnect_interval_secs** — How often to attempt reconnection (0 = disabled)
4. **connection_timeout_secs** — Timeout for peer connections

## Example: 3-Node Cluster

See `cluster_config_3node.toml` for a complete example.

### Setting Up Nodes

#### Node 1
```bash
# File: node1_config.toml
local_peer_id = "node-1"
auto_reconnect_interval_secs = 5
connection_timeout_secs = 10

[[peers]]
id = "node-1"
host = "localhost"
port = 9001

[[peers]]
id = "node-2"
host = "localhost"
port = 9002

[[peers]]
id = "node-3"
host = "localhost"
port = 9003
```

#### Node 2
Same file, but change:
```toml
local_peer_id = "node-2"
```

#### Node 3
Same file, but change:
```toml
local_peer_id = "node-3"
```

## Distributed Setup

For nodes on different machines:

```toml
# node1_config.toml (on 192.168.1.10)
local_peer_id = "node-1"
auto_reconnect_interval_secs = 5
connection_timeout_secs = 10

[[peers]]
id = "node-1"
host = "192.168.1.10"
port = 9001

[[peers]]
id = "node-2"
host = "192.168.1.20"
port = 9002

[[peers]]
id = "node-3"
host = "192.168.1.30"
port = 9003
```

## Loading Configuration in Code

```rust
use carbyne_db::config::DbReplicationConfig;

// Load config from file
let config = DbReplicationConfig::from_file("node1_config.toml")?;

// Validate configuration
config.validate()?;

// Get remote peers
let remotes = config.remote_peers();
println!("Connecting to {} peers", remotes.len());

// Get a specific peer
if let Some(peer) = config.get_peer("node-2") {
    println!("Node 2 at {}:{}", peer.host, peer.port);
}
```

## Configuration Validation

The config validates:
- `local_peer_id` is not empty
- `local_peer_id` exists in the peers list
- `peers` list is not empty
- Each peer has a valid id, host, and port

Validation errors are returned as `DbError::ConfigError`.

## Peer Connection Lifecycle

### Initial Connection
1. Load configuration
2. Get list of remote peers
3. Establish TCP connections (Phase 3.5)
4. Register peers with sync daemon

### Offline Handling
- Peer removed from daemon's peer registry
- Auto-reconnect timer (if enabled) attempts to re-establish
- Instance remains operational without peers

### Graceful Leave
- Remove all peers from daemon
- Continue accepting writes (local-only)
- Optionally snapshot state before leaving

### Restart
- Load configuration again
- Peers are NOT persisted (connections are ephemeral)
- Manually re-register peers with sync daemon

## Best Practices

1. **Use stable hostnames** — DNS names instead of IPs when possible
2. **Consistent peer IDs** — Use deterministic identifiers (e.g., "us-east-1-node-1")
3. **Port allocation** — Reserve port ranges per deployment (e.g., 9000-9099)
4. **Firewall rules** — Allow peer-to-peer traffic on configured ports
5. **Config backup** — Store configs in version control
6. **Graceful shutdown** — Snapshot state before stopping

## Troubleshooting

### Peer not connecting
- Verify host and port are accessible from this node
- Check firewall rules
- Confirm `local_peer_id` in config matches node identity

### Connection timeouts
- Increase `connection_timeout_secs` if network is slow
- Check network latency: `ping <host>`

### Config validation fails
- Verify `local_peer_id` matches one of the peer ids exactly
- Ensure all peer ids are unique
- Check all fields are present and non-empty
