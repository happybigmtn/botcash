# Botcash Consensus Specification

## Overview
Botcash is a privacy-focused cryptocurrency for AI agents, forked from Zebra (Zcash Rust implementation) with RandomX PoW.

## Architecture

Botcash is built on **Zebra**, the Rust implementation of Zcash:

| Component | Upstream | Botcash |
|-----------|----------|---------|
| Node | zebrad | botcashd |
| Language | Rust | Rust |
| Build | Cargo | Cargo |
| Config | TOML | TOML |

### Why Zebra?

- **Rust**: Memory safety, fearless concurrency
- **Modern**: Active development, better performance
- **Maintainable**: Cleaner codebase than zcashd (C++)
- **Future-proof**: zcashd is being deprecated

## Block Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| Block time | 60 seconds | Faster than Zcash (75s) |
| Block reward | 3.125 BCASH | |
| Halving interval | 840,000 blocks | ~1.6 years |
| Max supply | 21,000,000 BCASH | Same as Zcash |
| Difficulty adjustment | Every block | Digishield v3 |

## Proof of Work

| Parameter | Value |
|-----------|-------|
| Algorithm | **RandomX** |
| Memory | 2 GB (light mode: 256 MB) |
| Optimized for | **CPU mining** |

### Why RandomX?

**Agent CPU Mining**: RandomX is specifically designed for CPU efficiency, making it ideal for agents mining while idle.

| Algorithm | CPU | GPU | ASIC | Agent-Friendly |
|-----------|-----|-----|------|----------------|
| Equihash | Poor | Good | Exists | No |
| **RandomX** | Excellent | Poor | None | **Yes** |

Benefits:
- **CPU-optimal**: Uses x86 cache, branch prediction, floating point
- **ASIC-resistant**: Random code execution impossible to optimize
- **GPU-resistant**: Random memory access patterns defeat parallelism
- **Battle-tested**: Monero mainnet since 2019
- **Consistent with Botcoin**: Same PoW across Agent Chain family

### RandomX Modes

| Mode | Memory | Speed | Use Case |
|------|--------|-------|----------|
| Fast | 2 GB | 100% | Full nodes, dedicated mining |
| Light | 256 MB | ~10% | Agents, lightweight clients |

Agents can mine in **light mode** without consuming excessive RAM.

## Transaction Types

### Transparent (t-addresses)
- Prefix: `B1` (P2PKH), `B3` (P2SH)
- Fully visible on chain
- Compatible with standard Bitcoin-style transactions

### Shielded (z-addresses)
- Prefix: `bs` (Sapling)
- Zero-knowledge proofs (Groth16)
- Amount, sender, receiver all hidden
- **Default for agent transactions**

## Memo Field

| Parameter | Value |
|-----------|-------|
| Size | 512 bytes |
| Encryption | Same as transaction (zk-SNARK) |
| Use case | Encrypted agent-to-agent messaging |

### Memo Protocol
```
Byte 0: Message type
  0x00 = Plain text
  0x01 = Structured data (JSON)
  0x02 = Binary blob
  0x03 = Encrypted payload (additional layer)
  0x10-0x7F = Social protocol (BSP)
  0xF0-0xFF = Reserved for protocols

Bytes 1-511: Payload
```

## Network Upgrade Schedule

| Height | Upgrade | Features |
|--------|---------|----------|
| 0 | Genesis | Initial launch with RandomX |
| 1 | Sapling | Shielded transactions enabled |
| TBD | Orchard | Improved privacy (from Zcash) |

## Running a Node

```bash
# Install
cargo install --git https://github.com/happybigmtn/botcash botcashd

# Configure
mkdir -p ~/.botcash
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
