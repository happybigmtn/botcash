# AGENTS.md - Botcash Build Guide

## Overview

Botcash is forked from **Zebra** (Rust Zcash implementation), not zcashd (C++).

| Upstream | Botcash |
|----------|---------|
| zebrad | botcashd |
| Rust/Cargo | Rust/Cargo |
| TOML config | TOML config |

## Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Dependencies (Ubuntu/Debian)
sudo apt-get install build-essential pkg-config libssl-dev libclang-dev clang

# Dependencies (Arch Linux)
sudo pacman -S base-devel openssl clang

# Optional: protobuf for gRPC
sudo apt-get install protobuf-compiler  # Debian/Ubuntu
sudo pacman -S protobuf                  # Arch
```

## Build & Run

```bash
# Clone the repository
git clone https://github.com/happybigmtn/botcash
cd botcash

# Build release binary
cargo build --release

# Binary location
ls -la target/release/botcashd

# Run
./target/release/botcashd start
```

### Install from Git

```bash
# Install directly from repository
cargo install --git https://github.com/happybigmtn/botcash --tag v1.0.0 botcashd

# Or install from local checkout
cargo install --path .
```

## Validation

Run these after implementing to get immediate feedback:

```bash
# Build check (fast)
cargo check

# Full build
cargo build --release

# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with logging
RUST_LOG=debug cargo test

# Clippy lints
cargo clippy

# Format check
cargo fmt --check
```

## Binary

After build, binary is at `target/release/botcashd`:

```bash
# Start node
botcashd start

# Start with config file
botcashd -c ~/.botcash/botcash.toml start

# Generate default config
botcashd generate
```

## Configuration

Create `~/.botcash/botcash.toml`:

```toml
[network]
network = "Mainnet"
listen_addr = "0.0.0.0:8533"

[state]
cache_dir = "/home/user/.botcash/state"

[rpc]
listen_addr = "127.0.0.1:8532"

[mining]
miner_address = "bs1..."
threads = 4
```

## Project Structure

```
botcash/
├── Cargo.toml           # Workspace manifest
├── zebrad/              # Main binary crate (→ botcashd)
│   ├── src/
│   │   ├── main.rs
│   │   └── ...
│   └── Cargo.toml
├── zebra-chain/         # Blockchain primitives (→ botcash-chain)
├── zebra-consensus/     # Consensus rules (→ botcash-consensus)
├── zebra-network/       # P2P networking (→ botcash-network)
├── zebra-state/         # State management (→ botcash-state)
├── zebra-rpc/           # RPC server (→ botcash-rpc)
└── specs/               # Botcash specifications
```

## Key Files to Modify

### Branding (Phase 1)
- `Cargo.toml` - Package names, version
- `zebrad/Cargo.toml` - Binary name (zebrad → botcashd)
- All crate `Cargo.toml` files - Package names

### Consensus (Phase 2)
- `zebra-chain/src/parameters/network.rs` - Network enum (add Botcash)
- `zebra-chain/src/parameters/constants.rs` - Block time, rewards
- `zebra-consensus/src/block/subsidy.rs` - Block rewards, halving

### Network (Phase 3)
- `zebra-network/src/constants.rs` - Ports, magic bytes
- `zebra-network/src/config.rs` - Default config values
- `zebra-chain/src/parameters/network/magic.rs` - Network magic bytes

### PoW - RandomX (Phase 4)
- Add `randomx-rs` dependency
- `zebra-consensus/src/block/check.rs` - PoW verification
- `zebra-chain/src/work/` - Difficulty/work calculations

### Genesis (Phase 5)
- `zebra-chain/src/parameters/genesis.rs` - Genesis block
- `zebra-chain/src/block/genesis.rs` - Genesis creation

### Address Prefixes (Phase 6)
- `zebra-chain/src/transparent/address.rs` - t-address prefixes
- `zebra-chain/src/sapling/address.rs` - z-address prefixes

## Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --workspace --features=integration

# Specific crate tests
cargo test -p zebra-consensus

# With coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace
```

## Operational Notes

- Config file: `~/.botcash/botcash.toml`
- State directory: `~/.botcash/state/`
- Logs: stdout (use `RUST_LOG` env var)
- RPC: JSON-RPC 2.0 on port 8532

## RPC Examples

```bash
# Get block count
curl -s http://127.0.0.1:8532 -d '{
  "jsonrpc": "2.0",
  "method": "getblockcount",
  "params": [],
  "id": 1
}'

# Get network info
curl -s http://127.0.0.1:8532 -d '{
  "jsonrpc": "2.0",
  "method": "getinfo",
  "params": [],
  "id": 1
}'
```
