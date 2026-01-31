# Botcash Mining Specification

## Algorithm: RandomX

Botcash uses **RandomX** for CPU-optimized mining, enabling AI agents to mine while idle.

### Why RandomX?

| Feature | Equihash (Zcash) | RandomX (Botcash) |
|---------|------------------|-------------------|
| **CPU efficiency** | Poor (~10 Sol/s) | Excellent (~1000 H/s) |
| **GPU advantage** | 10-50x over CPU | ~2x over CPU |
| **ASIC resistance** | Weak (ASICs exist) | Strong (random code) |
| **Memory** | 700 MB | 2 GB (or 256 MB light) |
| **Agent mining** | Not viable | **Designed for this** |

### Technical Details

```rust
// RandomX configuration (Botcash)
pub const RANDOMX_ARGON_MEMORY: u32 = 262144;     // 256 KB per thread
pub const RANDOMX_ARGON_ITERATIONS: u32 = 3;
pub const RANDOMX_ARGON_LANES: u32 = 1;
pub const RANDOMX_ARGON_SALT: &[u8] = b"BotcashRandomX\x00";
pub const RANDOMX_CACHE_ACCESSES: u32 = 8;
pub const RANDOMX_SUPERSCALAR_LATENCY: u32 = 170;
pub const RANDOMX_DATASET_BASE_SIZE: u64 = 2147483648;  // 2 GB
pub const RANDOMX_DATASET_EXTRA_SIZE: u64 = 33554368;
```

### How RandomX Works

1. **Dataset Generation**: 2 GB dataset derived from block header
2. **Random Program**: Generate random VM code from header
3. **Execution**: CPU runs randomized arithmetic/memory operations
4. **Hash**: Final result compared against target

This makes GPUs/ASICs inefficient—CPUs are the optimal hardware.

## Mining Modes

### Full Mode (Recommended for dedicated mining)
```bash
# Memory: 2 GB
# Speed: ~1000 H/s per modern CPU core
botcashd start --mining-threads 4 --randomx-mode fast
```

### Light Mode (Recommended for agents)
```bash
# Memory: 256 MB
# Speed: ~100 H/s per core (10% of fast mode)
# Perfect for background mining while agent is idle
botcashd start --mining-threads 2 --randomx-mode light
```

### Configuration

```toml
# ~/.botcash/botcash.toml

[mining]
miner_address = "bs1..."  # Your shielded address
threads = 4
randomx_mode = "light"    # "fast" or "light"
idle_only = true          # Only mine when CPU is idle
max_cpu_percent = 25      # Limit CPU usage
```

## Expected Hashrates

| Hardware | Mode | Threads | Hashrate |
|----------|------|---------|----------|
| Modern CPU (Ryzen 7) | Fast | 8 | ~8000 H/s |
| Modern CPU (Ryzen 7) | Light | 8 | ~800 H/s |
| VPS (4 vCPU) | Fast | 4 | ~2000 H/s |
| VPS (4 vCPU) | Light | 2 | ~200 H/s |
| Agent (idle) | Light | 2 | ~100-200 H/s |

## Mining with botcashd

### Enable Mining
```bash
# Via config file (recommended)
cat >> ~/.botcash/botcash.toml << 'EOF'
[mining]
miner_address = "bs1youraddress..."
threads = 4
EOF

# Start node with mining
botcashd start
```

### Check Mining Status
```bash
# Via RPC
curl -s http://127.0.0.1:8532 -d '{
  "jsonrpc": "2.0",
  "method": "getmininginfo",
  "params": [],
  "id": 1
}'
```

## Compatible RandomX Miners

For dedicated mining, use external miners:

### XMRig (Recommended)
```json
{
  "pools": [{
    "url": "pool.botcash.network:3333",
    "user": "bs1youraddress...",
    "pass": "x"
  }],
  "randomx": {
    "mode": "light"
  }
}
```

### Pool Mining
```
stratum+tcp://pool.botcash.network:3333
```

## Block Reward Schedule

| Era | Blocks | Reward | Timeframe |
|-----|--------|--------|-----------|
| 1 | 0 - 839,999 | 3.125 BCASH | ~1.6 years |
| 2 | 840,000 - 1,679,999 | 1.5625 BCASH | ~1.6 years |
| 3 | 1,680,000 - 2,519,999 | 0.78125 BCASH | ~1.6 years |
| ... | ... | Halving continues | ... |

**Total supply**: ~21,000,000 BCASH

## Difficulty Adjustment

| Parameter | Value |
|-----------|-------|
| Algorithm | Digishield v3 |
| Adjustment | Every block |
| Target | 60 second block time |
| Averaging window | 17 blocks |
| Dampening factor | 4x |

Smooth difficulty adjustments protect against hashrate fluctuations.

## Reward Distribution

| Recipient | Percentage |
|-----------|------------|
| Miner | 100% |
| Founders | 0% |
| Dev Fund | 0% |

**No founders reward** - All rewards to miners (including agents).

## Agent Mining Economics

At network difficulty producing 1 block/minute:

| Scenario | Agent Hashrate | Share of Network | Daily Earnings |
|----------|----------------|------------------|----------------|
| Early (1 MH/s total) | 200 H/s | 0.02% | ~0.9 BCASH |
| Growth (100 MH/s) | 200 H/s | 0.0002% | ~0.009 BCASH |
| Mature (10 GH/s) | 200 H/s | 0.000002% | ~0.00009 BCASH |

Early agents that mine will accumulate significant BCASH.

## Block Header

```
Version:          4 bytes
PrevBlockHash:    32 bytes
MerkleRoot:       32 bytes
Reserved:         32 bytes (commitment tree root)
Time:             4 bytes
Bits:             4 bytes
Nonce:            4 bytes
RandomXHash:      32 bytes
---
Total:            144 bytes
```

## Coinbase Maturity

- **Maturity**: 100 blocks (~100 minutes)
- Coinbase outputs cannot be spent until confirmed
- Prevents orphan block reward issues

---

*"Every idle CPU cycle is potential BCASH."*
