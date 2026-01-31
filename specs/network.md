# Botcash Network Specification

## Network Ports

| Network | P2P Port | RPC Port |
|---------|----------|----------|
| Mainnet | 8533 | 8532 |
| Testnet | 18533 | 18532 |
| Regtest | 18544 | 18543 |

## Magic Bytes

| Network | Magic |
|---------|-------|
| Mainnet | `0x42 0x43 0x41 0x53` ("BCAS") |
| Testnet | `0x54 0x42 0x43 0x41` ("TBCA") |

## DNS Seeds
```
seed1.botcash.network
seed2.botcash.network
seed3.botcash.network
```

## Protocol Version
- Base: 170100 (from Zcash/Zebra)
- Botcash: 180100

## Node User Agent
```
/Botcash:1.0.0/
```

## Configuration

### botcash.toml

```toml
# ~/.botcash/botcash.toml

[network]
# Network to connect to: "Mainnet", "Testnet", or "Regtest"
network = "Mainnet"

# P2P listen address
listen_addr = "0.0.0.0:8533"

# Initial peers (optional, DNS seeds used by default)
initial_mainnet_peers = [
    "95.111.227.14:8533",
    "185.239.209.227:8533"
]

# Peer connection limits
peerset_initial_target_size = 25

[state]
# State cache directory
cache_dir = "/home/user/.botcash/state"

# Enable state on disk (vs memory-only)
ephemeral = false

[rpc]
# RPC listen address (localhost only by default)
listen_addr = "127.0.0.1:8532"

# Enable debug RPC methods
debug_force_finished_sync = false

[sync]
# Checkpoint sync for faster initial sync
checkpoint_sync = true

[tracing]
# Log level: "off", "error", "warn", "info", "debug", "trace"
filter = "info"
```

## Peer Discovery

1. **DNS seeds** (mainnet) - Primary discovery method
2. **Initial peers** (config) - Hardcoded bootstrap nodes
3. **Peer exchange** - addr/getaddr messages
4. **Manual peers** - Via config or RPC

## Seed Nodes (Bootstrap)
```
95.111.227.14:8533   # Genesis node
185.239.209.227:8533 # Backup
```

## Running a Node

### Quick Start
```bash
# Install
cargo install --git https://github.com/happybigmtn/botcash botcashd

# Create config directory
mkdir -p ~/.botcash

# Start with defaults
botcashd start
```

### With Custom Config
```bash
# Create config
cat > ~/.botcash/botcash.toml << 'EOF'
[network]
network = "Mainnet"
listen_addr = "0.0.0.0:8533"

[state]
cache_dir = "/home/user/.botcash/state"

[rpc]
listen_addr = "127.0.0.1:8532"
EOF

# Start
botcashd start
```

### Check Node Status
```bash
# Via RPC
curl -s http://127.0.0.1:8532 -d '{
  "jsonrpc": "2.0",
  "method": "getinfo",
  "params": [],
  "id": 1
}'
```

## Health Endpoints

botcashd exposes health endpoints for monitoring:

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | Basic liveness check |
| `GET /ready` | Readiness (synced) check |

Useful for Kubernetes probes and load balancers.

## Firewall Configuration

```bash
# Allow P2P connections
sudo ufw allow 8533/tcp

# RPC should remain localhost only unless needed
# sudo ufw allow from 10.0.0.0/8 to any port 8532
```
