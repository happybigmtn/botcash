# Botcash Branding

## Binary Names

| Zebra | Botcash |
|-------|---------|
| zebrad | botcashd |

**Note**: Unlike zcashd, Zebra doesn't have a separate CLI tool. RPC commands are sent via JSON-RPC (curl, zcash-rpc-cli, or the built-in RPC client).

## Installation

```bash
# From source (Botcash fork)
cargo install --git https://github.com/happybigmtn/botcash --tag v1.0.0 botcashd

# Or build locally
git clone https://github.com/happybigmtn/botcash
cd botcash
cargo build --release
# Binary at: target/release/botcashd
```

## Data Directory

| OS | Botcash |
|----|---------|
| Linux | ~/.botcash |
| macOS | ~/Library/Application Support/Botcash |
| Windows | %APPDATA%\Botcash |

## Configuration File

Botcash uses TOML configuration (inherited from Zebra):

```toml
# ~/.botcash/botcash.toml

[network]
network = "Mainnet"
listen_addr = "0.0.0.0:8533"

[state]
cache_dir = "~/.botcash/state"

[rpc]
listen_addr = "127.0.0.1:8532"

[mining]
miner_address = "B1..."  # Your transparent address
```

## Currency

| Property | Value |
|----------|-------|
| Symbol | BCASH |
| Name | Botcash |
| Smallest unit | zatoshi → batoshi |
| Decimals | 8 |

## RPC Usage

```bash
# Check block height
curl -X POST http://127.0.0.1:8532 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}'

# Get network info
curl -X POST http://127.0.0.1:8532 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getinfo","params":[],"id":1}'
```

## bcash-cli Wrapper (Optional)

For convenience, create a shell wrapper:

```bash
#!/bin/bash
# ~/.local/bin/bcash-cli
curl -s -X POST http://127.0.0.1:8532 \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"$1\",\"params\":[${@:2}],\"id\":1}" \
  | jq -r '.result // .error'
```
