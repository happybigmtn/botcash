# Botcash Implementation Plan

> **Monorepo** containing all components for Botcash: Privacy + Social blockchain for AI agents.
> Based on Zcash ecosystem: Zebra (node), librustzcash (libraries), lightwalletd (backend), Zashi (wallets).

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MOBILE CLIENTS                                 │
│  ┌─────────────────────┐              ┌─────────────────────┐           │
│  │   zashi-ios/        │              │   zashi-android/    │           │
│  │   (Swift)           │              │   (Kotlin)          │           │
│  │   → Botcash Wallet  │              │   → Botcash Wallet  │           │
│  └──────────┬──────────┘              └──────────┬──────────┘           │
│             │                                    │                       │
│             └──────────────┬─────────────────────┘                       │
│                            │ gRPC (port 9067)                            │
│             ┌──────────────▼──────────────┐                              │
│             │      lightwalletd/          │                              │
│             │      (Go)                   │                              │
│             │      → botcash-lightwalletd │                              │
│             └──────────────┬──────────────┘                              │
│                            │ JSON-RPC (port 8532)                        │
│             ┌──────────────▼──────────────┐                              │
│             │      zebrad/ → botcashd     │                              │
│             │      (Rust)                 │                              │
│             │      Full Node + RandomX    │                              │
│             └─────────────────────────────┘                              │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                      librustzcash/                               │    │
│  │  Core Rust libraries: zcash_protocol, zcash_address, etc.       │    │
│  │  → Defines network constants, address prefixes, consensus       │    │
│  └─────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Component Summary

| Component | Directory | Language | Purpose | Upstream |
|-----------|-----------|----------|---------|----------|
| **Full Node** | `zebrad/`, `zebra-*/` | Rust | Blockchain node, RandomX PoW | Zebra v2.5.0 |
| **Rust Libraries** | `librustzcash/` | Rust | Address encoding, network params | librustzcash |
| **Light Wallet Server** | `lightwalletd/` | Go | Mobile backend, block streaming | lightwalletd |
| **iOS Wallet** | `zashi-ios/` | Swift | Mobile wallet app | Zashi iOS |
| **Android Wallet** | `zashi-android/` | Kotlin | Mobile wallet app | Zashi Android |
| **Specifications** | `specs/` | Markdown | Protocol documentation | — |

---

## Implementation Phases

### Phase 0: Network Constants (librustzcash) — FOUNDATION
> **Must be done first.** All other components depend on these definitions.

#### 0.1 Add Botcash Network Type
**Files:**
- `librustzcash/components/zcash_protocol/src/consensus.rs`

**Changes:**
```rust
// Line ~130: Add to NetworkType enum
pub enum NetworkType {
    Main,
    Test,
    Regtest,
    Botcash,      // NEW
    BotcashTest,  // NEW
}
```

**Required Tests:**
```rust
#[test]
fn botcash_network_type_exists() {
    assert!(matches!(NetworkType::Botcash, NetworkType::Botcash));
    assert!(matches!(NetworkType::BotcashTest, NetworkType::BotcashTest));
}
```

---

#### 0.2 Create Botcash Constants Module
**Files to CREATE:**
- `librustzcash/components/zcash_protocol/src/constants/botcash.rs`

**Content:**
```rust
/// Botcash mainnet constants
pub const COIN_TYPE: u32 = 347;  // Register with SLIP-44

// Sapling HRP prefixes
pub const HRP_SAPLING_EXTENDED_SPENDING_KEY: &str = "secret-extended-key-botcash";
pub const HRP_SAPLING_EXTENDED_FULL_VIEWING_KEY: &str = "bviews";
pub const HRP_SAPLING_PAYMENT_ADDRESS: &str = "bs";

// TEX address
pub const HRP_TEX_ADDRESS: &str = "btex";

// Unified address HRPs
pub const HRP_UNIFIED_ADDRESS: &str = "bu";
pub const HRP_UNIFIED_FVK: &str = "buview";
pub const HRP_UNIFIED_IVK: &str = "buivk";

// Base58Check prefixes (transparent addresses)
pub const B58_PUBKEY_ADDRESS_PREFIX: [u8; 2] = [0x19, 0x1C];  // "B1"
pub const B58_SCRIPT_ADDRESS_PREFIX: [u8; 2] = [0x19, 0x21];  // "B3"
pub const B58_SECRET_KEY_PREFIX: [u8; 1] = [0x80];

// Legacy Sprout (not used but required)
pub const B58_SPROUT_ADDRESS_PREFIX: [u8; 2] = [0x16, 0x9a];
```

**Files to MODIFY:**
- `librustzcash/components/zcash_protocol/src/constants.rs` — Add `pub mod botcash;`

**Required Tests:**
```rust
#[test]
fn botcash_sapling_address_hrp() {
    assert_eq!(botcash::HRP_SAPLING_PAYMENT_ADDRESS, "bs");
}

#[test]
fn botcash_transparent_prefix_is_b1() {
    // B58 decode should start with "B1"
    let prefix = botcash::B58_PUBKEY_ADDRESS_PREFIX;
    assert_eq!(prefix, [0x19, 0x1C]);
}
```

---

#### 0.3 Implement NetworkConstants Trait for Botcash
**Files:**
- `librustzcash/components/zcash_protocol/src/consensus.rs`

**Changes:** Add match arms in NetworkType impl:
```rust
impl NetworkConstants for NetworkType {
    fn coin_type(&self) -> u32 {
        match self {
            // ... existing ...
            NetworkType::Botcash => constants::botcash::COIN_TYPE,
            NetworkType::BotcashTest => 1,
        }
    }

    fn hrp_sapling_payment_address(&self) -> &'static str {
        match self {
            // ... existing ...
            NetworkType::Botcash => constants::botcash::HRP_SAPLING_PAYMENT_ADDRESS,
            NetworkType::BotcashTest => "bstest",
        }
    }
    // ... implement all trait methods ...
}
```

**Required Tests:**
```rust
#[test]
fn botcash_network_constants_complete() {
    let net = NetworkType::Botcash;
    assert_eq!(net.coin_type(), 347);
    assert_eq!(net.hrp_sapling_payment_address(), "bs");
    assert_eq!(net.hrp_unified_address(), "bu");
}
```

---

#### 0.4 Update Address Encoding
**Files:**
- `librustzcash/components/zcash_address/src/encoding.rs`

**Changes:** Add Botcash HRP cases in address parsing (3 locations):
```rust
// Sapling address parsing (~line 80)
constants::botcash::HRP_SAPLING_PAYMENT_ADDRESS => NetworkType::Botcash,

// TEX address parsing (~line 105)
constants::botcash::HRP_TEX_ADDRESS => NetworkType::Botcash,

// Base58 prefix parsing (~line 125)
```

**Files:**
- `librustzcash/components/zcash_address/src/kind/unified/address.rs`
- `librustzcash/components/zcash_address/src/kind/unified/fvk.rs`
- `librustzcash/components/zcash_address/src/kind/unified/ivk.rs`

**Changes:** Add BOTCASH constant to SealedContainer implementations.

**Required Tests:**
```rust
#[test]
fn parse_botcash_sapling_address() {
    let addr = "bs1qtest...";  // valid encoded address
    let parsed = ZcashAddress::try_from_encoded(addr);
    assert!(parsed.is_ok());
    assert_eq!(parsed.unwrap().network(), NetworkType::Botcash);
}

#[test]
fn encode_botcash_transparent_address() {
    let addr = TransparentAddress::new_p2pkh(/* ... */);
    let encoded = addr.encode(&NetworkType::Botcash);
    assert!(encoded.starts_with("B1"));
}
```

---

### Phase 1: Full Node (Zebra) — Core Blockchain

#### 1.1 Add Botcash Network Variant
**Files:**
- `zebra-chain/src/parameters/network.rs`

**Changes:**
```rust
pub enum Network {
    Mainnet,
    Testnet,
    Botcash,      // NEW
    BotcashTest,  // NEW
}
```

**Required Tests:**
```rust
#[test]
fn botcash_network_variant() {
    let network = Network::Botcash;
    assert_eq!(network.to_string(), "Botcash");
}
```

---

#### 1.2 Network Magic Bytes
**Files:**
- `zebra-chain/src/parameters/constants.rs`
- `zebra-chain/src/parameters/network/magic.rs`

**Changes:**
```rust
pub const BOTCASH_MAGIC: Magic = Magic([0x42, 0x43, 0x41, 0x53]); // "BCAS"
pub const BOTCASH_TEST_MAGIC: Magic = Magic([0x54, 0x42, 0x43, 0x41]); // "TBCA"
```

**Required Tests:**
```rust
#[test]
fn botcash_magic_is_bcas() {
    assert_eq!(&BOTCASH_MAGIC.0, b"BCAS");
}
```

---

#### 1.3 Network Ports
**Files:**
- `zebra-chain/src/parameters/network.rs` (line ~237)
- `zebra-network/src/config.rs`

**Changes:**
```rust
// P2P ports
pub const BOTCASH_MAINNET_PORT: u16 = 8533;
pub const BOTCASH_TESTNET_PORT: u16 = 18533;

// RPC ports (in zebra-rpc)
pub const BOTCASH_RPC_MAINNET_PORT: u16 = 8532;
pub const BOTCASH_RPC_TESTNET_PORT: u16 = 18532;
```

**Required Tests:**
```rust
#[test]
fn botcash_default_ports() {
    let config = Config::default_for(Network::Botcash);
    assert_eq!(config.network.listen_addr.port(), 8533);
}
```

---

#### 1.4 Block Time (60 seconds)
**Files:**
- `zebra-chain/src/parameters/constants.rs`

**Changes:**
```rust
pub const BOTCASH_POW_TARGET_SPACING: Duration = Duration::from_secs(60);
```

**Required Tests:**
```rust
#[test]
fn botcash_block_time_is_60_seconds() {
    assert_eq!(BOTCASH_POW_TARGET_SPACING, Duration::from_secs(60));
}
```

---

#### 1.5 Block Reward (3.125 BCASH, 100% to miners)
**Files:**
- `zebra-consensus/src/block/subsidy.rs`

**Changes:**
```rust
pub fn botcash_block_subsidy(height: Height) -> Amount {
    let initial_subsidy = Amount::from_zatoshis(312_500_000); // 3.125 BCASH
    let halvings = height.0 / 840_000;
    if halvings >= 64 {
        return Amount::zero();
    }
    initial_subsidy >> (halvings as u32)
}

// NO founders reward - 100% to miners
pub fn botcash_miner_subsidy(height: Height) -> Amount {
    botcash_block_subsidy(height)
}
```

**Required Tests:**
```rust
#[test]
fn botcash_subsidy_schedule() {
    assert_eq!(botcash_block_subsidy(Height(1)).to_zatoshis(), 312_500_000);
    assert_eq!(botcash_block_subsidy(Height(840_000)).to_zatoshis(), 156_250_000);
    assert_eq!(botcash_block_subsidy(Height(1_680_000)).to_zatoshis(), 78_125_000);
}

#[test]
fn botcash_no_founders_reward() {
    let height = Height(100);
    assert_eq!(botcash_miner_subsidy(height), botcash_block_subsidy(height));
}
```

---

#### 1.6 RandomX PoW Integration
**Files:**
- `Cargo.toml` (workspace)
- `zebra-consensus/Cargo.toml`
- `zebra-consensus/src/block/check.rs`
- `zebra-chain/src/work/randomx.rs` (NEW)

**Changes:**
```toml
# Cargo.toml
[dependencies]
randomx-rs = "1.2"
```

```rust
// zebra-chain/src/work/randomx.rs
use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlags, RandomXVM};

pub fn verify_randomx_pow(
    header: &BlockHeader,
    target: &CompactDifficulty,
    key_block_hash: &Hash,
) -> Result<(), VerifyError> {
    let flags = RandomXFlags::default();
    let cache = RandomXCache::new(flags, key_block_hash.as_bytes())?;
    let vm = RandomXVM::new(flags, Some(&cache), None)?;

    let hash = vm.hash(&header.serialize());
    let hash_value = U256::from_le_bytes(hash);
    let target_value = target.to_target().to_u256();

    if hash_value > target_value {
        return Err(VerifyError::InvalidPoW);
    }
    Ok(())
}
```

**Required Tests:**
```rust
#[test]
fn randomx_pow_verification() {
    let valid_header = create_valid_botcash_header();
    assert!(verify_randomx_pow(&valid_header, &target, &key_hash).is_ok());
}

#[test]
fn randomx_rejects_invalid_pow() {
    let invalid_header = create_invalid_header();
    assert!(verify_randomx_pow(&invalid_header, &target, &key_hash).is_err());
}
```

---

#### 1.7 Genesis Block
**Files:**
- `zebra-chain/src/parameters/genesis.rs`
- `zebra-chain/src/block/genesis/block-botcash-0-000-000.txt` (NEW)

**Changes:**
```rust
pub const BOTCASH_GENESIS_MESSAGE: &str =
    "Privacy is not secrecy. Agents deserve both.";

pub fn botcash_genesis_block() -> Block {
    // Genesis block parameters:
    // - Timestamp: TBD (launch date)
    // - Nonce: TBD (mined with RandomX)
    // - Message in coinbase: BOTCASH_GENESIS_MESSAGE
    // - Target: Initial difficulty
}
```

**Required Tests:**
```rust
#[test]
fn botcash_genesis_is_valid() {
    let genesis = botcash_genesis_block();
    assert!(verify_block(&genesis, Network::Botcash).is_ok());
}

#[test]
fn botcash_genesis_contains_message() {
    let genesis = botcash_genesis_block();
    let coinbase = genesis.coinbase_tx();
    assert!(coinbase.memo().contains("Agents deserve both"));
}
```

---

#### 1.8 Address Prefixes (Transparent)
**Files:**
- `zebra-chain/src/transparent/address.rs`

**Changes:**
```rust
impl TransparentAddress {
    pub fn encode(&self, network: &Network) -> String {
        let prefix = match (self, network) {
            (Self::PayToPublicKeyHash(_), Network::Botcash) => [0x19, 0x1C], // "B1"
            (Self::PayToScriptHash(_), Network::Botcash) => [0x19, 0x21],    // "B3"
            // ... existing cases ...
        };
        bs58::encode_check(&[&prefix[..], self.hash()].concat())
    }
}
```

**Required Tests:**
```rust
#[test]
fn botcash_p2pkh_starts_with_b1() {
    let addr = TransparentAddress::new_p2pkh(hash);
    let encoded = addr.encode(&Network::Botcash);
    assert!(encoded.starts_with("B1"));
}

#[test]
fn botcash_p2sh_starts_with_b3() {
    let addr = TransparentAddress::new_p2sh(hash);
    let encoded = addr.encode(&Network::Botcash);
    assert!(encoded.starts_with("B3"));
}
```

---

### Phase 2: Light Wallet Server (lightwalletd)

#### 2.1 Network Parameters
**Files to CREATE:**
- `lightwalletd/common/network_params.go`

**Content:**
```go
package common

type NetworkParams struct {
    Name                    string
    RPCDefaultPort          string
    TaddrPrefixRegex        string
    SaplingActivationHeight uint64
}

var Networks = map[string]*NetworkParams{
    "main":         {Name: "main", RPCDefaultPort: "8232", TaddrPrefixRegex: "^t1"},
    "test":         {Name: "test", RPCDefaultPort: "18232", TaddrPrefixRegex: "^tm"},
    "botcash":      {Name: "botcash", RPCDefaultPort: "8532", TaddrPrefixRegex: "^B1"},
    "botcash-test": {Name: "botcash-test", RPCDefaultPort: "18532", TaddrPrefixRegex: "^B1"},
}
```

---

#### 2.2 RPC Client Updates
**Files:**
- `lightwalletd/frontend/rpc_client.go`

**Changes:**
```go
// Line ~65: Add Botcash port detection
func getRPCPort(conf map[string]string) string {
    if conf["botcash"] == "1" {
        return "8532"
    }
    if conf["testnet"] == "1" {
        return "18232"
    }
    return "8232"
}
```

---

#### 2.3 Address Validation
**Files:**
- `lightwalletd/frontend/service.go`

**Changes:**
```go
// Line ~60: Update address regex
func checkTaddress(taddr string, network *NetworkParams) error {
    pattern := network.TaddrPrefixRegex + "[a-zA-Z0-9]{33}$"
    match, err := regexp.Match(pattern, []byte(taddr))
    // ...
}
```

---

#### 2.4 Chain Name Detection
**Files:**
- `lightwalletd/cmd/root.go`
- `lightwalletd/common/common.go`

**Changes:**
```go
// Detect Botcash from getblockchaininfo response
chainName := response.Chain
if chainName == "botcash" || chainName == "botcash-test" {
    // Use Botcash network params
}
```

**Required Tests:**
```go
func TestBotcashNetworkDetection(t *testing.T) {
    params := Networks["botcash"]
    assert.Equal(t, "8532", params.RPCDefaultPort)
    assert.Equal(t, "^B1", params.TaddrPrefixRegex)
}
```

---

### Phase 3: iOS Wallet (zashi-ios)

#### 3.1 Network Configuration
**Files:**
- `zashi-ios/modules/Sources/Dependencies/ZcashSDKEnvironment/ZcashSDKEnvironmentInterface.swift`

**Changes:**
```swift
// Update default endpoints
public static let endpointMainnetAddress = "botcash-mainnet.example.com"
public static let endpointTestnetAddress = "botcash-testnet.example.com"
public static let endpointMainnetPort = 9067
public static let endpointTestnetPort = 19067
```

---

#### 3.2 Build Targets
**Files:**
- `zashi-ios/secant/SecantApp.swift`

**Changes:**
```swift
// Add BOTCASH_MAINNET compiler flag
#if BOTCASH_MAINNET
    public static var tokenName: String { "BCASH" }
#elseif BOTCASH_TESTNET
    public static var tokenName: String { "tBCASH" }
#endif
```

---

#### 3.3 Branding
**Files:**
- `zashi-ios/secant/secant-mainnet-Info.plist`
- `zashi-ios/modules/Sources/Generated/Resources/Assets.xcassets/`

**Changes:**
- Update `CFBundleDisplayName` to "Botcash"
- Replace Zashi icons with Botcash icons
- Update background task identifiers

---

### Phase 4: Android Wallet (zashi-android)

#### 4.1 Server Configuration
**Files:**
- `zashi-android/ui-lib/src/main/java/co/electriccoin/zcash/ui/common/provider/LightWalletEndpointProvider.kt`

**Changes:**
```kotlin
fun getEndpoints(network: ZcashNetwork): List<LightWalletEndpoint> {
    return when (network) {
        ZcashNetwork.Mainnet -> listOf(
            LightWalletEndpoint("botcash-mainnet.example.com", 9067, true)
        )
        ZcashNetwork.Testnet -> listOf(
            LightWalletEndpoint("botcash-testnet.example.com", 19067, true)
        )
    }
}
```

---

#### 4.2 Build Flavors
**Files:**
- `zashi-android/build-conventions-secant/src/main/kotlin/model/Dimensions.kt`

**Changes:**
```kotlin
enum class NetworkDimension(val flavorName: String) {
    MAINNET("botcashmainnet"),
    TESTNET("botcashtestnet")
}
```

---

#### 4.3 Branding
**Files:**
- `zashi-android/gradle.properties`

**Changes:**
```properties
ZCASH_RELEASE_APP_NAME=Botcash
ZCASH_RELEASE_PACKAGE_NAME=com.botcash.wallet
```

---

### Phase 5: Social Protocol (Application Layer)

#### 5.1 Memo Protocol Parser
**Files:**
- `zebra-chain/src/transaction/memo/social.rs` (NEW)

**Changes:**
```rust
#[repr(u8)]
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

pub struct SocialMessage {
    pub version: u8,
    pub msg_type: SocialMessageType,
    pub payload: Vec<u8>,
}

impl TryFrom<&Memo> for SocialMessage {
    type Error = SocialParseError;

    fn try_from(memo: &Memo) -> Result<Self, Self::Error> {
        let bytes = memo.as_bytes();
        if bytes.is_empty() {
            return Err(SocialParseError::Empty);
        }
        // Parse based on first byte (message type)
        // ...
    }
}
```

**Required Tests:**
```rust
#[test]
fn parse_post_message() {
    let memo = create_post_memo("Hello Botcash!");
    let msg = SocialMessage::try_from(&memo).unwrap();
    assert_eq!(msg.msg_type, SocialMessageType::Post);
}

#[test]
fn parse_dm_message() {
    let memo = create_dm_memo("Private message");
    let msg = SocialMessage::try_from(&memo).unwrap();
    assert_eq!(msg.msg_type, SocialMessageType::DM);
}
```

---

#### 5.2 Social RPC Extensions
**Files:**
- `zebra-rpc/src/methods/social.rs` (NEW)

**Methods:**
```rust
// z_socialpost - Create a social post
pub async fn z_socialpost(
    &self,
    from: String,
    content: String,
    tags: Vec<String>,
) -> Result<TxId, RpcError>;

// z_socialdm - Send encrypted DM
pub async fn z_socialdm(
    &self,
    from: String,
    to: String,
    content: String,
) -> Result<TxId, RpcError>;

// z_socialfollow - Follow a user
pub async fn z_socialfollow(
    &self,
    from: String,
    target: String,
) -> Result<TxId, RpcError>;

// z_socialfeed - Get feed from viewing keys
pub async fn z_socialfeed(
    &self,
    ivks: Vec<String>,
    limit: u32,
) -> Result<Vec<SocialPost>, RpcError>;
```

---

#### 5.3 Mobile Social UI
**Files (iOS):**
- `zashi-ios/modules/Sources/Features/Social/` (NEW directory)
  - `FeedView.swift`
  - `FeedStore.swift`
  - `PostView.swift`
  - `MessageView.swift`

**Files (Android):**
- `zashi-android/ui-lib/src/main/java/co/electriccoin/zcash/ui/screen/social/` (NEW)
  - `FeedScreen.kt`
  - `FeedVM.kt`
  - `PostScreen.kt`

---

## Implementation Order

```
Phase 0: librustzcash (MUST BE FIRST)
    ├── 0.1 NetworkType enum
    ├── 0.2 Botcash constants module
    ├── 0.3 NetworkConstants trait impl
    └── 0.4 Address encoding

Phase 1: Zebra (Full Node)
    ├── 1.1 Network variant
    ├── 1.2 Magic bytes
    ├── 1.3 Ports
    ├── 1.4 Block time
    ├── 1.5 Block reward
    ├── 1.6 RandomX PoW
    ├── 1.7 Genesis block
    └── 1.8 Address prefixes

Phase 2: lightwalletd
    ├── 2.1 Network params
    ├── 2.2 RPC client
    ├── 2.3 Address validation
    └── 2.4 Chain detection

Phase 3: iOS Wallet
    ├── 3.1 Network config
    ├── 3.2 Build targets
    └── 3.3 Branding

Phase 4: Android Wallet
    ├── 4.1 Server config
    ├── 4.2 Build flavors
    └── 4.3 Branding

Phase 5: Social Protocol
    ├── 5.1 Memo parser
    ├── 5.2 Social RPC
    └── 5.3 Mobile UI
```

---

## Testing Commands

```bash
# librustzcash
cd librustzcash && cargo test --workspace

# Zebra (full node)
cargo test --workspace
cargo build --release
./target/release/botcashd --help

# lightwalletd
cd lightwalletd && go test ./...
go build -o botcash-lightwalletd .

# iOS (requires Xcode)
cd zashi-ios && xcodebuild test -scheme Botcash-Mainnet

# Android
cd zashi-android && ./gradlew test
./gradlew assembleBotcashmainnetFossDebug
```

---

## Key Specifications

| Parameter | Value |
|-----------|-------|
| **Currency** | BCASH |
| **PoW Algorithm** | RandomX (CPU-optimized) |
| **Block Time** | 60 seconds |
| **Initial Reward** | 3.125 BCASH |
| **Halving Interval** | 840,000 blocks (~1.6 years) |
| **Max Supply** | ~21M BCASH |
| **P2P Port** | 8533 (mainnet), 18533 (testnet) |
| **RPC Port** | 8532 (mainnet), 18532 (testnet) |
| **Transparent Prefix** | B1 (P2PKH), B3 (P2SH) |
| **Shielded Prefix** | bs (Sapling), bu (Unified) |
| **Founders Reward** | None (100% to miners) |
| **Social Protocol** | BSP (Botcash Social Protocol) |

---

## Notes

1. **Dependency Order**: librustzcash changes MUST be made first, as all other Rust components depend on these network definitions.

2. **SDK Updates**: Mobile SDKs (zcash-swift-wallet-sdk, zcash-android-sdk) will need to be forked or configured to use Botcash network parameters.

3. **Sapling Parameters**: Reuse Zcash's trusted setup. No ceremony needed.

4. **Genesis Mining**: After all parameters are set, mine genesis block using RandomX.

5. **Social Protocol**: Built on top of the existing 512-byte memo field. No consensus changes required.
