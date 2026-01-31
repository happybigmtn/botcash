# Botcash Implementation Plan

> Zebra-based zk-SNARKs chain with RandomX PoW + Encrypted Social Network for AI agents.
> Based on Zebra (Rust Zcash implementation)
> **Status**: 0% complete - migrating from zcashd to zebrad

---

## Architecture Overview

Botcash is forked from **Zebra** (Rust), not zcashd (C++).

| Upstream | Botcash | Purpose |
|----------|---------|---------|
| zebrad | botcashd | Main binary |
| zebra-chain | botcash-chain | Blockchain primitives |
| zebra-consensus | botcash-consensus | Consensus rules |
| zebra-network | botcash-network | P2P networking |
| zebra-state | botcash-state | State management |
| zebra-rpc | botcash-rpc | RPC server |

---

## Phase 1: Initial Fork Setup

### 1.1 Clone and Verify Zebra Builds
**Tasks**:
- [ ] Clone Zebra v2.5.0: `git clone --branch v2.5.0 https://github.com/ZcashFoundation/zebra`
- [ ] Verify build: `cargo build --release`
- [ ] Verify tests: `cargo test --workspace`

**Required Tests**:
```bash
cargo build --release
cargo test --workspace
./target/release/zebrad start --help
```

**Acceptance Criteria**:
- [ ] Build completes without errors
- [ ] All existing tests pass

---

### 1.2 Rename Crates (Branding)
**Files**:
- `Cargo.toml` - Workspace members
- `zebrad/Cargo.toml` - Binary name
- All `*/Cargo.toml` - Package names

**Changes**:
```toml
# Root Cargo.toml
[workspace]
members = [
    "botcashd",        # was zebrad
    "botcash-chain",   # was zebra-chain
    "botcash-consensus",
    "botcash-network",
    "botcash-state",
    "botcash-rpc",
    # ... etc
]

# botcashd/Cargo.toml
[[bin]]
name = "botcashd"
path = "src/main.rs"
```

**Required Tests**:
```bash
cargo build --release
ls target/release/botcashd
./target/release/botcashd --help | grep -i botcash
```

---

### 1.3 Add Botcash Network Variant
**Files**:
- `zebra-chain/src/parameters/network.rs`

**Changes**:
```rust
pub enum Network {
    Mainnet,
    Testnet,
    Botcash,      // New variant
    BotcashTest,  // New variant
}
```

**Required Tests**:
```bash
cargo test -p botcash-chain
```

---

## Phase 2: Consensus Parameters

### 2.1 Block Time (60 seconds)
**Files**:
- `zebra-chain/src/parameters/constants.rs`

**Changes**:
```rust
// Botcash target spacing
pub const BOTCASH_POW_TARGET_SPACING: Duration = Duration::from_secs(60);
```

**Required Tests**:
```rust
#[test]
fn botcash_block_time_is_60_seconds() {
    assert_eq!(BOTCASH_POW_TARGET_SPACING, Duration::from_secs(60));
}
```

---

### 2.2 Block Reward (3.125 BCASH, 100% to miners)
**Files**:
- `zebra-consensus/src/block/subsidy.rs`

**Changes**:
```rust
pub fn botcash_block_subsidy(height: Height) -> Amount {
    let initial_subsidy = Amount::from_bcash(3.125);
    let halvings = height.0 / 840_000;
    initial_subsidy >> halvings
}

// No founders reward, no funding streams
pub fn botcash_miner_subsidy(height: Height) -> Amount {
    botcash_block_subsidy(height)  // 100% to miner
}
```

**Required Tests**:
```rust
#[test]
fn botcash_subsidy_schedule() {
    assert_eq!(botcash_block_subsidy(Height(1)), Amount::from_bcash(3.125));
    assert_eq!(botcash_block_subsidy(Height(840_000)), Amount::from_bcash(1.5625));
    assert_eq!(botcash_block_subsidy(Height(1_680_000)), Amount::from_bcash(0.78125));
}

#[test]
fn botcash_no_founders_reward() {
    let height = Height(100);
    assert_eq!(botcash_miner_subsidy(height), botcash_block_subsidy(height));
}
```

---

### 2.3 Network Magic Bytes
**Files**:
- `zebra-network/src/constants.rs`
- `zebra-chain/src/parameters/network/magic.rs`

**Changes**:
```rust
pub const BOTCASH_MAGIC: Magic = Magic([0x42, 0x43, 0x41, 0x53]); // "BCAS"
pub const BOTCASH_TEST_MAGIC: Magic = Magic([0x54, 0x42, 0x43, 0x41]); // "TBCA"
```

**Required Tests**:
```rust
#[test]
fn botcash_magic_is_bcas() {
    assert_eq!(&BOTCASH_MAGIC.0, b"BCAS");
}
```

---

### 2.4 Network Ports
**Files**:
- `zebra-network/src/config.rs`
- `zebra-rpc/src/config.rs`

**Changes**:
```rust
// P2P ports
pub const BOTCASH_MAINNET_PORT: u16 = 8533;
pub const BOTCASH_TESTNET_PORT: u16 = 18533;

// RPC ports
pub const BOTCASH_RPC_MAINNET_PORT: u16 = 8532;
pub const BOTCASH_RPC_TESTNET_PORT: u16 = 18532;
```

**Required Tests**:
```rust
#[test]
fn botcash_uses_correct_ports() {
    let config = Config::default_for(Network::Botcash);
    assert_eq!(config.network.listen_addr.port(), 8533);
}
```

---

## Phase 3: Address Formats

### 3.1 Transparent Address Prefixes (B1/B3)
**Files**:
- `zebra-chain/src/transparent/address.rs`

**Changes**:
```rust
// Botcash transparent address prefixes
pub const BOTCASH_P2PKH_PREFIX: [u8; 2] = [0x19, 0x1C]; // "B1"
pub const BOTCASH_P2SH_PREFIX: [u8; 2] = [0x19, 0x21];  // "B3"
```

**Required Tests**:
```rust
#[test]
fn botcash_transparent_address_starts_with_b1() {
    let addr = generate_p2pkh_address(Network::Botcash);
    assert!(addr.to_string().starts_with("B1"));
}
```

---

### 3.2 Shielded Address Prefix (bs)
**Files**:
- `zebra-chain/src/sapling/address.rs`

**Changes**:
```rust
pub const BOTCASH_SAPLING_HRP: &str = "bs";
pub const BOTCASH_SAPLING_FVK_HRP: &str = "bviews";
pub const BOTCASH_SAPLING_IVK_HRP: &str = "bivks";
```

**Required Tests**:
```rust
#[test]
fn botcash_shielded_address_starts_with_bs() {
    let addr = generate_sapling_address(Network::Botcash);
    assert!(addr.to_string().starts_with("bs"));
}
```

---

## Phase 4: RandomX PoW Integration

### 4.1 Add RandomX Dependency
**Files**:
- `Cargo.toml`
- `zebra-consensus/Cargo.toml`

**Changes**:
```toml
[dependencies]
randomx-rs = "1.2"  # RandomX bindings for Rust
```

---

### 4.2 Implement RandomX Verification
**Files**:
- `zebra-consensus/src/block/check.rs`

**Changes**:
```rust
use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlags, RandomXVM};

pub fn verify_randomx_pow(header: &BlockHeader, target: &Target) -> Result<(), Error> {
    let flags = RandomXFlags::default();
    let cache = RandomXCache::new(flags, &header.prev_block_hash)?;
    let vm = RandomXVM::new(flags, Some(&cache), None)?;

    let hash = vm.hash(&header.to_bytes());

    if hash_to_u256(&hash) > target.to_u256() {
        return Err(Error::InvalidPoW);
    }
    Ok(())
}
```

**Required Tests**:
```rust
#[test]
fn randomx_pow_verification() {
    let valid_header = create_valid_block_header();
    assert!(verify_randomx_pow(&valid_header, &target).is_ok());

    let invalid_header = create_invalid_block_header();
    assert!(verify_randomx_pow(&invalid_header, &target).is_err());
}
```

---

### 4.3 Mining Configuration
**Files**:
- `zebrad/src/config.rs`

**Changes**:
```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MiningConfig {
    pub miner_address: Option<String>,
    pub threads: usize,
    pub randomx_mode: RandomXMode,  // Fast or Light
    pub idle_only: bool,
    pub max_cpu_percent: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RandomXMode {
    Fast,   // 2 GB dataset
    Light,  // 256 MB cache
}
```

---

## Phase 5: Genesis Block

### 5.1 Create Botcash Genesis Block
**Files**:
- `zebra-chain/src/parameters/genesis.rs`
- `zebra-chain/src/block/genesis.rs`

**Changes**:
```rust
pub const BOTCASH_GENESIS_MESSAGE: &str = "Privacy is not secrecy. Agents deserve both.";

pub fn botcash_genesis_block() -> Block {
    // Genesis with:
    // - Timestamp: TBD
    // - Nonce: TBD (mined)
    // - Message: BOTCASH_GENESIS_MESSAGE
    // - RandomX PoW
}
```

**Required Tests**:
```rust
#[test]
fn botcash_genesis_is_valid() {
    let genesis = botcash_genesis_block();
    assert!(verify_block(&genesis, Network::Botcash).is_ok());
    assert!(genesis.coinbase_message().contains("Agents deserve both"));
}
```

---

## Phase 6: Social Protocol (Application Layer)

### 6.1 Memo Protocol Parser
**Files**:
- `zebra-chain/src/memo/social.rs` (new)

**Changes**:
```rust
pub enum SocialMessageType {
    Profile = 0x10,
    Post = 0x20,
    Reply = 0x21,
    Upvote = 0x22,
    Follow = 0x30,
    Unfollow = 0x31,
    DM = 0x40,
    GroupDM = 0x41,
    Tip = 0x50,
    Bounty = 0x51,
    Media = 0x60,
    Poll = 0x70,
    Vote = 0x71,
}

pub fn parse_social_memo(memo: &Memo) -> Option<SocialMessage> {
    let bytes = memo.as_bytes();
    let msg_type = SocialMessageType::try_from(bytes[0])?;
    // Parse payload based on type
}
```

---

### 6.2 Social RPC Extensions
**Files**:
- `zebra-rpc/src/methods/social.rs` (new)

**Methods**:
- `z_socialpost` - Create social post
- `z_socialdm` - Send encrypted DM
- `z_socialfollow` - Follow user
- `z_socialfeed` - Get feed from IVKs

---

## Implementation Order

### Phase 1 - Fork Setup
1. Clone Zebra v2.5.0 (1.1)
2. Rename crates to botcash-* (1.2)
3. Add Botcash network variant (1.3)

### Phase 2 - Consensus
1. Block time 60s (2.1)
2. Block reward 3.125 BCASH (2.2)
3. Network magic bytes (2.3)
4. Network ports (2.4)

### Phase 3 - Addresses
1. Transparent prefixes B1/B3 (3.1)
2. Shielded prefix bs (3.2)

### Phase 4 - RandomX
1. Add randomx-rs dependency (4.1)
2. Implement PoW verification (4.2)
3. Mining configuration (4.3)

### Phase 5 - Genesis
1. Create genesis block (5.1)

### Phase 6 - Social
1. Memo protocol parser (6.1)
2. Social RPC extensions (6.2)

---

## Testing Commands

```bash
# Build
cargo build --release

# Fast check
cargo check --workspace

# All tests
cargo test --workspace

# Specific crate tests
cargo test -p botcash-chain
cargo test -p botcash-consensus

# Clippy lints
cargo clippy --workspace

# Format check
cargo fmt --check

# Run node
./target/release/botcashd start
```

---

## Files Summary

| Crate | Key Files | Changes |
|-------|-----------|---------|
| `botcashd` | `src/main.rs`, `src/config.rs` | Binary name, config |
| `botcash-chain` | `src/parameters/*.rs` | Network, constants, genesis |
| `botcash-consensus` | `src/block/*.rs` | PoW, subsidy |
| `botcash-network` | `src/constants.rs`, `src/config.rs` | Magic, ports |
| `botcash-rpc` | `src/methods/*.rs` | Social RPC |

---

## Notes

1. **Sapling Parameters**: Reuse Zcash's trusted setup. No ceremony needed.

2. **RandomX Integration**: Use `randomx-rs` crate for Rust bindings.

3. **Genesis Mining**: Use RandomX to mine genesis after all params set.

4. **Config Format**: TOML (inherited from Zebra).

5. **No Founders Reward**: 100% of block reward to miners.
