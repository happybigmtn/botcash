# Botcash Implementation Plan

> **Monorepo** containing all components for Botcash: Privacy + Social blockchain for AI agents.
> Based on Zcash ecosystem: Zebra (node), librustzcash (libraries), lightwalletd (backend), Zashi (wallets).

---

## 🚦 Current Status: PHASES 0-5 COMPLETE, PHASE 6 IN PROGRESS

**Last Updated:** 2026-02-01 (P6.1a Complete - Transaction Batching Core)

Phase 0 (librustzcash network constants and address encoding) is complete. Phase 1 (Zebra Full Node) is **COMPLETE**: P1.1-P1.15 all done. Phase 2 (lightwalletd Go Backend) is **COMPLETE**: P2.1-P2.5 all done. Phase 3 (iOS Wallet) is **COMPLETE**: P3.1-P3.7 all done (endpoint updates, bundle identifiers, CFBundleDisplayName, background task identifiers, app icons with Botcash "B" branding, and localization strings updated to Botcash/BCASH). Phase 4 (Android Wallet) is **COMPLETE**: P4.1-P4.4 all done. Phase 5 (Social Protocol) is **COMPLETE**: P5.1-P5.10 all done (SocialMessageType enum with 17 types including attention market types and Batch, SocialMessage struct, TryFrom<&Memo>, pub mod social, social RPC methods z_socialpost/z_socialdm/z_socialfollow/z_socialfeed, RPC response types, attention market governance parameters, attention RPC methods z_attentionboost/z_credittip/z_creditbalance/z_marketfeed/z_epochstats with validation, and full Rpc trait with all methods). Phase 6 (Infrastructure) is **IN PROGRESS**: P6.1a done (Batch message type 0x80 with BatchMessage struct, MAX_BATCH_ACTIONS=5, encode/decode roundtrip, 14 tests).

**Key Finding:** 744 TODO/FIXME markers across 181 files; 18 HIGH relevance to Botcash implementation.

---

## 📋 Priority Task List (Sorted by Dependency Order)

### ✅ Phase 0: librustzcash (COMPLETE)

All other phases depend on Phase 0. These tasks define the network identity.

| Priority | Task | Status | Files (with line numbers) | Test Command |
|----------|------|--------|---------------------------|--------------|
| **P0.1** | Add `NetworkType::Botcash` enum | ✅ DONE | `librustzcash/components/zcash_protocol/src/consensus.rs:131-141` | `cd librustzcash && cargo test -p zcash_protocol -- botcash` |
| **P0.2** | Create botcash.rs constants (12 constants) | ✅ DONE | `librustzcash/components/zcash_protocol/src/constants/botcash.rs` | `cd librustzcash && cargo test -p zcash_protocol -- botcash` |
| **P0.3** | Add `pub mod botcash;` to constants.rs | ✅ DONE | `librustzcash/components/zcash_protocol/src/constants.rs:1-6` | `cd librustzcash && cargo test -p zcash_protocol -- botcash` |
| **P0.4** | Implement `NetworkConstants` trait (12 match arms) | ✅ DONE | `librustzcash/components/zcash_protocol/src/consensus.rs:236-330` | `cd librustzcash && cargo test -p zcash_protocol -- botcash` |
| **P0.5** | Update Sapling address parsing | ✅ DONE | `librustzcash/components/zcash_address/src/encoding.rs:76-86` | `cd librustzcash && cargo test -p zcash_address -- botcash` |
| **P0.6** | Update TEX address parsing | ✅ DONE | `librustzcash/components/zcash_address/src/encoding.rs:100-108` | `cd librustzcash && cargo test -p zcash_address -- botcash` |
| **P0.7** | Update Base58Check prefix parsing | ✅ DONE | `librustzcash/components/zcash_address/src/encoding.rs:123-131` | `cd librustzcash && cargo test -p zcash_address -- botcash` |
| **P0.8** | Extend SealedContainer trait for Botcash | ✅ DONE | `librustzcash/components/zcash_address/src/kind/unified.rs:209-236` | `cd librustzcash && cargo test -p zcash_address -- botcash` |
| **P0.9** | Update Unified Address container | ✅ DONE | `librustzcash/components/zcash_address/src/kind/unified/address.rs:137-158` | `cd librustzcash && cargo test -p zcash_address -- botcash` |
| **P0.10** | Update Unified FVK container | ✅ DONE | `librustzcash/components/zcash_address/src/kind/unified/fvk.rs:132-146` | `cd librustzcash && cargo test -p zcash_address -- botcash` |
| **P0.11** | Update Unified IVK container | ✅ DONE | `librustzcash/components/zcash_address/src/kind/unified/ivk.rs:137-147` | `cd librustzcash && cargo test -p zcash_address -- botcash` |

**Tests passing:**
- `cargo test -p zcash_protocol -- botcash` → 2 tests pass
- `cargo test -p zcash_address -- botcash` → 4 tests pass

### ✅ Phase 1: Zebra Full Node (Core Blockchain) — COMPLETE

| Priority | Task | Status | Files (with line numbers) | Test Command |
|----------|------|--------|---------------------------|--------------|
| **P1.1** | Add `NetworkKind::Botcash` variant | ✅ DONE | `zebra-chain/src/parameters/network.rs:26-39` | `cargo test -p zebra-chain -- network_kind` |
| **P1.2** | Add `Network::Botcash` variant | ✅ DONE | `zebra-chain/src/parameters/network.rs:53-67` | `cargo test -p zebra-chain -- botcash_network_variant` |
| **P1.3** | Add BOTCASH magic bytes (0x42434153) | ✅ DONE | `zebra-chain/src/parameters/constants.rs:29-30` | `cargo test -p zebra-chain -- magic` |
| **P1.4** | Update Network::magic() impl | ✅ DONE | `zebra-chain/src/parameters/network/magic.rs:21-29` | `cargo test -p zebra-chain -- network_magic` |
| **P1.5** | Set network ports (8533/18533) | ✅ DONE | `zebra-chain/src/parameters/network.rs:251-260` | `cargo test -p zebra-chain -- default_port` |
| **P1.6** | Set block time (60s) | ✅ DONE | `zebra-chain/src/parameters/network_upgrade.rs:294-296` | `cargo test -p zebra-chain -- block_time` |
| **P1.7** | Implement block subsidy (3.125 BCASH) | ✅ DONE | `zebra-chain/src/parameters/network/subsidy.rs:30-40,800-815` | `cargo test -p zebra-chain -- botcash_subsidy` |
| **P1.8** | Disable funding streams for Botcash | ✅ DONE | `zebra-chain/src/parameters/network/testnet.rs:918-943` | `cargo test -p zebra-chain -- no_funding` |
| **P1.9** | Add randomx-rs dependency | ✅ DONE | `Cargo.toml:62` (workspace deps) | `cargo check -p zebra-chain` |
| **P1.10** | Create RandomX verification module | ✅ DONE | `zebra-chain/src/work/randomx.rs` (299 lines) | `cargo test -p zebra-chain -- randomx` |
| **P1.11** | Add `pub mod randomx;` to work.rs | ✅ DONE | `zebra-chain/src/work.rs:5` | `cargo check -p zebra-chain` |
| **P1.12** | Integrate RandomX in block check | ✅ DONE | `zebra-consensus/src/block/check.rs:141-232` | `cargo test -p zebra-consensus -- pow_solution` |
| **P1.13** | Update VerifyBlockError enum | ✅ DONE | `zebra-consensus/src/block.rs:69-80,109-128` | `cargo test -p zebra-consensus -- randomx` |
| **P1.14** | Create genesis block function | ✅ DONE | `zebra-chain/src/block/genesis.rs:24-53` | `cargo test -p zebra-chain -- botcash_genesis` |
| **P1.15** | Update transparent address encoding | ✅ DONE | `zebra-chain/src/transparent/address.rs:190-245` | `cargo test -p zebra-chain -- transparent_address` |

### ✅ Phase 2: lightwalletd (Go Backend) — COMPLETE

| Priority | Task | Status | Files (with line numbers) | Test Command |
|----------|------|--------|---------------------------|--------------|
| **P2.1** | Create network_params.go | ✅ DONE | `lightwalletd/common/network_params.go` (299 lines) | `cd lightwalletd && go test ./common/... -run TestNetwork` |
| **P2.2** | Update RPC port detection | ✅ DONE | `lightwalletd/frontend/rpc_client.go:49-67` | `cd lightwalletd && go test ./frontend/...` |
| **P2.3** | Update address validation regex | ✅ DONE | `lightwalletd/frontend/service.go:55-82` | `cd lightwalletd && go test ./frontend/... -run TestGetTaddress` |
| **P2.4** | Add Botcash chain name detection | ✅ DONE | `lightwalletd/cmd/root.go:217-229` | `cd lightwalletd && go test ./cmd/...` |
| **P2.5** | Update NodeName detection | ✅ DONE | `lightwalletd/cmd/root.go:222-229` | `cd lightwalletd && go test ./cmd/...` |

### ✅ Phase 3: iOS Wallet (zashi-ios) — COMPLETE

| Priority | Task | Status | Files | Test Command |
|----------|------|--------|-------|--------------|
| **P3.1** | Update endpoint constants (9 URLs) | ✅ DONE | `zashi-ios/modules/Sources/Dependencies/ZcashSDKEnvironment/ZcashSDKEnvironmentInterface.swift:24-27,94-106` | `xcodebuild test -scheme Botcash -only-testing:SecantTests/NetworkTests` |
| **P3.2** | Update legacy migration paths | ✅ DONE | `zashi-ios/modules/Sources/Dependencies/ZcashSDKEnvironment/ZcashSDKEnvironmentLiveKey.swift:89,93` | Build verification |
| **P3.3** | Update bundle identifiers (6 targets) | ✅ DONE | `zashi-ios/secant.xcodeproj/project.pbxproj` | `xcodebuild -showBuildSettings` |
| **P3.4** | Update CFBundleDisplayName (5 plists) | ✅ DONE | `zashi-ios/secant/*-Info.plist:12-13` | Visual verification |
| **P3.5** | Update background task identifiers | ✅ DONE | `zashi-ios/secant/AppDelegate.swift:20-21` | Build verification |
| **P3.6** | Replace app icons (3 iconsets) | ✅ DONE | `zashi-ios/secant/Resources/Assets.xcassets/AppIcon*.appiconset/` | Visual verification |
| **P3.7** | Update localization strings (~50 refs) | ✅ DONE | `zashi-ios/modules/Sources/Generated/Resources/*/Localizable.strings` | String verification |

### ✅ Phase 4: Android Wallet (zashi-android) — COMPLETE

| Priority | Task | Status | Files | Test Command |
|----------|------|--------|-------|--------------|
| **P4.1** | Update endpoint list | ✅ DONE | `zashi-android/ui-lib/src/main/java/co/electriccoin/zcash/ui/common/provider/LightWalletEndpointProvider.kt:14-30` | `./gradlew :ui-lib:testDebugUnitTest --tests "*EndpointTest*"` |
| **P4.2** | Update network flavor dimension | ✅ DONE | `zashi-android/build-conventions-secant/src/main/kotlin/model/Dimensions.kt` | `./gradlew assembleBotcashmainnetDebug` |
| **P4.3** | Update gradle.properties branding | ✅ DONE | `zashi-android/gradle.properties:69-70` | `./gradlew :app:lintDebug` |
| **P4.4** | Create botcash flavor directories | ✅ DONE | `zashi-android/app/src/botcash*/` (7 directories renamed) | Build verification |

### ✅ Phase 5: Social Protocol (Application Layer) — COMPLETE

| Priority | Task | Status | Files | Test Command |
|----------|------|--------|-------|--------------|
| **P5.1** | Create SocialMessageType enum (16 types) | ✅ DONE | `zebra-chain/src/transaction/memo/social.rs` | `cargo test -p zebra-chain -- social_message_type` |
| **P5.2** | Create SocialMessage struct | ✅ DONE | `zebra-chain/src/transaction/memo/social.rs` | `cargo test -p zebra-chain -- social_message` |
| **P5.3** | Implement TryFrom<&Memo> for SocialMessage | ✅ DONE | `zebra-chain/src/transaction/memo/social.rs` | `cargo test -p zebra-chain -- memo_parse` |
| **P5.4** | Add `pub mod social;` to memo.rs | ✅ DONE | `zebra-chain/src/transaction/memo.rs` | `cargo build -p zebra-chain` |
| **P5.5** | Create social RPC methods (4 methods) | ✅ DONE | `zebra-rpc/src/methods.rs:682-757,3001-3170` | `cargo test -p zebra-rpc -- z_social` |
| **P5.6** | Create social RPC response types | ✅ DONE | `zebra-rpc/src/methods/types/social.rs` (NEW) | `cargo test -p zebra-rpc -- social_types` |
| **P5.7** | Add attention message types (0x52-0x54) | ✅ DONE | `zebra-chain/src/transaction/memo/social.rs` | `cargo test -p zebra-chain -- attention_boost` |
| **P5.8** | Create attention RPC methods (5 methods) | ✅ DONE | `zebra-rpc/src/methods.rs:760-886,3309-3559` | `cargo test -p zebra-rpc -- z_attention` |
| **P5.9** | Create attention parameters | ✅ DONE | `zebra-chain/src/parameters/attention.rs` (NEW) | `cargo test -p zebra-chain -- attention_params` |
| **P5.10** | Update methods.rs Rpc trait | ✅ DONE | `zebra-rpc/src/methods.rs:132-886` | `cargo check -p zebra-rpc` |

### ✅ Phase 6: Infrastructure (Post-Launch) — IN PROGRESS

| Priority | Task | Status | Files | Test Command |
|----------|------|--------|-------|--------------|
| **P6.1a** | Batch message type (0x80) | ✅ DONE | `zebra-chain/src/transaction/memo/social.rs` | `cargo test -p zebra-chain -- batch` |
| **P6.1b** | Wallet batch queue | ⬜ TODO | See specs/scaling.md | TBD |
| **P6.1c** | Indexer batch parsing | ⬜ TODO | See specs/scaling.md | TBD |
| **P6.2** | Layer-2 channels | ⬜ TODO | See specs/scaling.md | TBD |
| **P6.3** | Governance voting | ⬜ TODO | See specs/governance.md | TBD |
| **P6.4** | Social recovery | ⬜ TODO | See specs/recovery.md | TBD |
| **P6.5** | Platform bridges | ⬜ TODO | See specs/bridges.md | TBD |

**P6.1a Implementation Details:**
- Added `SocialMessageType::Batch = 0x80` for batched transactions
- Created `BatchMessage` struct with `MAX_BATCH_ACTIONS = 5` actions limit
- Implemented binary encoding: `[0x80][version][count][len_lo][len_hi][action]...`
- Added `BatchParseError` for batch-specific error handling
- Nested batches are explicitly forbidden
- 14 comprehensive tests covering roundtrip, max actions, mixed types, error cases

---

## 🔍 Deep Codebase Analysis (2026-01-31)

### Current State Summary

| Component | Location | Status | Gap Analysis |
|-----------|----------|--------|--------------|
| **librustzcash** | `librustzcash/` | Unmodified Zcash | `NetworkType` has only Main/Test/Regtest (line 131-141 in consensus.rs) |
| **Zebra node** | `zebra-*/` | Unmodified Zcash | `Network` has only Mainnet/Testnet (line 53-61 in network.rs) |
| **RandomX** | — | **NOT PRESENT** | No randomx-rs dependency; uses Equihash (equihash.rs: 298 lines) |
| **Genesis block** | `zebra-chain/src/parameters/genesis.rs:7` | Zcash only | Only defines GENESIS_PREVIOUS_BLOCK_HASH |
| **Block subsidy** | `zebra-chain/src/parameters/network/subsidy.rs:30` | Zcash (12.5 ZEC) | MAX_BLOCK_SUBSIDY hardcoded, needs 3.125 BCASH |
| **Address encoding** | `librustzcash/.../encoding.rs:76-131` | Zcash prefixes (t1, zs) | 3 match statements need Botcash cases |
| **Magic bytes** | `zebra-chain/src/parameters/constants.rs:20-29` | Zcash (0x24e92764) | No BCAS (0x42434153) defined |
| **Ports** | `zebra-chain/src/parameters/network.rs:236-241` | 8233/18233 | default_port() needs Botcash case |
| **lightwalletd** | `lightwalletd/frontend/rpc_client.go:59-66` | Zcash ports | Hardcoded 8232/18232 |
| **iOS wallet** | `zashi-ios/.../ZcashSDKEnvironmentInterface.swift:24-27` | Zashi branding | 9 zec.rocks endpoints hardcoded |
| **Android wallet** | `zashi-android/.../LightWalletEndpointProvider.kt:14-30` | Zashi branding | 8 zec.rocks endpoints hardcoded |
| **Social protocol** | — | **NOT PRESENT** | No memo/social.rs module |
| **Attention market** | — | **NOT PRESENT** | No attention.rs module |

### Key Files to Modify (Phase 0-1) with Line Numbers

```
librustzcash/components/zcash_protocol/src/
├── consensus.rs          ← Add NetworkType::Botcash (lines 131-141)
│                         ← Extend NetworkConstants impl (lines 236-330, 12 methods)
├── constants.rs          ← Add `pub mod botcash;` (line 4)
└── constants/
    ├── mainnet.rs        (reference: 12 constants defined)
    ├── testnet.rs        (reference: 12 constants defined)
    ├── regtest.rs        (reference: 12 constants defined)
    └── botcash.rs        ← CREATE: COIN_TYPE=347, 11 HRP/prefix constants

librustzcash/components/zcash_address/src/
├── encoding.rs           ← Add Botcash cases at lines 76-86, 100-108, 123-131
└── kind/unified/
    ├── address.rs        ← Add BOTCASH const (lines 137-158)
    ├── fvk.rs            ← Add BOTCASH const (lines 132-146)
    └── ivk.rs            ← Add BOTCASH const (lines 137-147)

zebra-chain/src/parameters/
├── network.rs            ← Add NetworkKind::Botcash (line 35), default_port() (line 237)
├── constants.rs          ← Add BOTCASH_MAGIC (after line 29), BOTCASH_POW_TARGET_SPACING
├── genesis.rs            ← Add botcash_genesis_block() function
└── network/
    ├── subsidy.rs        ← Add Botcash subsidy at line 30, halving logic at 421-483
    ├── magic.rs          ← Add BOTCASH magic constant (lines 21-28)
    └── testnet.rs        ← Reference for custom network parameters

zebra-chain/src/work/
├── equihash.rs           (existing Zcash PoW, 298 lines)
├── work.rs               ← Add `pub mod randomx;` after line 4
└── randomx.rs            ← CREATE: RandomX verification (~200-300 lines)

zebra-consensus/src/block/
├── check.rs              ← Add randomx_solution_is_valid() at line 141-149
└── ../block.rs           ← Update VerifyBlockError at lines 69-73
```

### RandomX Integration Points

**Current Equihash Flow (to replace for Botcash):**
1. `zebra-chain/src/work/equihash.rs:70-92` - `Solution::check()` validates PoW
2. `zebra-consensus/src/block/check.rs:141-149` - `equihash_solution_is_valid()` entry point
3. `zebra-consensus/src/block.rs:209` - Called from block verification

**RandomX Required Changes:**
- Add `randomx-rs` to workspace `Cargo.toml:61` (next to `equihash = "0.2.2"`)
- Create `zebra-chain/src/work/randomx.rs` mirroring equihash.rs structure
- Add network-aware dispatch in `check.rs` to call RandomX for Botcash

### Existing Test Infrastructure

**Test Patterns:**
- Unit tests: `fn test_{functionality}()` or `fn {description}()`
- Property tests: `proptest! { #[test] fn prop_{desc}() }`
- Vector tests: `fn {functionality}_test_vectors()`
- All tests require: `let _init_guard = zebra_test::init();`

**Test Commands:**
- Single test: `cargo test -p {crate} -- {test_name}`
- Module tests: `cargo test -p {crate} -- {module}::`
- With logging: `RUST_LOG=debug cargo test -- {test_name}`
- Property cases: `PROPTEST_CASES=100 cargo test -- {test_name}`

**Existing Test Counts:**
- **4,233+ Rust tests** across zebra-* crates
- **45 Go tests** in lightwalletd
- **No Botcash-specific tests** exist yet

### High-Relevance TODOs in Codebase (18 found)

| File | Line | TODO | Impact |
|------|------|------|--------|
| `equihash.rs` | 73-77 | Add Equihash parameters field to testnet::Parameters | Blocks configurable PoW |
| `equihash.rs` | 117 | Accept network as argument for Regtest | Needed for Botcash variant |
| `miner.rs` | 3-7 | Pause mining if no peers, add developer config | Mining behavior |
| `miner.rs` | 105 | Spawn new executor for mining isolation | Performance |
| `mining.rs` | 19,43 | Internal miner config removed | Needs reimplementation |
| `subsidy.rs` | 295-339 | 5 TODOs about funding streams and ZIP refs | Botcash subsidy logic |
| `network.rs` | 24,230,239 | Testnet params, history tree, funding | Custom network config |
| `network_upgrade.rs` | 499 | Move TESTNET_MINIMUM_DIFFICULTY_START_HEIGHT | Difficulty scheduling |
| `testnet.rs` | 522,618,743 | Parameter serialization and funding | Botcash testnet config |

### lightwalletd Hardcoded References (12 files)

| File | Lines | Reference | Change Needed |
|------|-------|-----------|---------------|
| `rpc_client.go` | 59-66 | Port 8232/18232 | Add 8532/18532 for Botcash |
| `service.go` | 56-67 | Regex `\\At[a-zA-Z0-9]{34}\\z` | Add B1/bs prefix support |
| `root.go` | 40-42,344 | "zcash.conf", "Zcash" strings | Update for Botcash |
| `common.go` | 32 | `NodeName = "zebrad"` | Add Botcash detection |

### Mobile Wallet Hardcoded References

**iOS (zashi-ios):**
- `ZcashSDKEnvironmentInterface.swift:24-27,94-106` - 9 endpoint URLs
- `secant/*-Info.plist:12-13` - 5 CFBundleDisplayName entries
- `AppDelegate.swift:20-21` - 2 background task identifiers
- `Localizable.strings` - ~50 "Zashi" string references

**Android (zashi-android):**
- `LightWalletEndpointProvider.kt:14-30` - 8 endpoint URLs
- `Dimensions.kt` - Network flavor enum ("zcashmainnet", "zcashtestnet")
- `gradle.properties:63-72` - Package name, app name

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

#### 5.4 Attention Market (specs/attention-market.md)

> **Circular attention economy**: Paid rankings redistributed as tip credits with 7-day expiration.

##### 5.4.1 Core Transaction Types
**Files:**
- `zebra-chain/src/transaction/memo/social.rs`

**Changes:**
```rust
// Add new message types
pub enum SocialMessageType {
    // ... existing types ...
    AttentionBoost = 0x52,  // Pay to boost content visibility
    CreditTip = 0x53,       // Tip using credits (not BCASH)
    CreditClaim = 0x54,     // Claim earned credits from pool
}

#[derive(Debug, Clone)]
pub struct AttentionBoost {
    pub target_txid: TxId,
    pub duration_blocks: u32,
    pub category: u8,
}

#[derive(Debug, Clone)]
pub struct CreditTip {
    pub target_txid: TxId,
    pub credit_amount: Amount,
    pub message: Option<String>,
}
```

**Required Tests:**
```bash
cargo test -p zebra-chain test_attention_boost_memo_parse
cargo test -p zebra-chain test_credit_tip_memo_parse
```

---

##### 5.4.2 Credit Pool & Redistribution Logic
**Files:**
- `botcash-indexer/src/credits.rs` (NEW)
- `botcash-indexer/src/epoch.rs` (NEW)

**Changes:**
```rust
// Credit balance tracking
pub struct CreditBalance {
    pub address: ZcashAddress,
    pub balance: Amount,
    pub grants: Vec<CreditGrant>,
}

pub struct CreditGrant {
    pub amount: Amount,
    pub granted_block: Height,
    pub expires_block: Height,  // granted + 10080 (7 days)
    pub spent: Amount,
}

// Epoch pool for redistribution
pub struct Epoch {
    pub number: u32,
    pub start_block: Height,
    pub end_block: Height,
    pub total_paid: Amount,
    pub payers: HashMap<ZcashAddress, Amount>,
}

impl Epoch {
    /// Calculate credits earned by each payer
    /// redistribution_rate = 0.8 (80% redistributed)
    pub fn calculate_credits(&self) -> HashMap<ZcashAddress, Amount> {
        let pool = self.total_paid * 80 / 100;
        self.payers.iter()
            .map(|(addr, paid)| {
                let share = pool * paid / self.total_paid;
                (addr.clone(), share)
            })
            .collect()
    }
}
```

**Required Tests:**
```bash
cargo test -p botcash-indexer test_credit_redistribution
cargo test -p botcash-indexer test_credit_expiration
cargo test -p botcash-indexer test_epoch_calculation
```

---

##### 5.4.3 Market Ranking Algorithm
**Files:**
- `botcash-indexer/src/market.rs` (NEW)

**Changes:**
```rust
/// Attention Units calculation
pub fn calculate_au(content: &MarketContent) -> f64 {
    let paid_weight = 1.0;
    let tip_weight = 2.0;  // Tips worth 2x (organic signal)

    (content.bcash_paid.to_f64() * paid_weight) +
    (content.tips_received.to_f64() * tip_weight)
}

/// Time-decayed ranking for "hot" feed
pub fn calculate_rank(content: &MarketContent, current_block: Height) -> f64 {
    let base_au = calculate_au(content);
    let age_blocks = current_block.0 - content.boost_start_block.0;
    let half_life = 1440.0;  // 1 day in blocks
    let decay = 0.5_f64.powf(age_blocks as f64 / half_life);

    let boost_multiplier = if content.boost_end_block > current_block {
        1.5
    } else {
        1.0
    };

    base_au * decay * boost_multiplier
}
```

**Required Tests:**
```bash
cargo test -p botcash-indexer test_au_calculation
cargo test -p botcash-indexer test_rank_decay
cargo test -p botcash-indexer test_boost_multiplier
```

---

##### 5.4.4 RPC Extensions
**Files:**
- `zebra-rpc/src/methods/attention.rs` (NEW)

**Methods:**
```rust
/// Boost content visibility
pub async fn z_attentionboost(
    &self,
    from: String,
    target_txid: String,
    amount: Amount,
    duration_blocks: u32,
    category: Option<u8>,
) -> Result<TxId, RpcError>;

/// Tip using credits (not BCASH)
pub async fn z_credittip(
    &self,
    from: String,
    target_txid: String,
    credit_amount: Amount,
    message: Option<String>,
) -> Result<TxId, RpcError>;

/// Get credit balance with expiration info
pub async fn z_creditbalance(
    &self,
    address: String,
) -> Result<CreditBalanceResponse, RpcError>;

/// Get market feed
pub async fn z_marketfeed(
    &self,
    feed_type: String,  // "hot", "top", "new", "boosted"
    category: Option<u8>,
    limit: u32,
    offset: u32,
) -> Result<Vec<MarketContent>, RpcError>;

/// Get epoch statistics
pub async fn z_epochstats(
    &self,
    epoch_number: Option<u32>,  // None = current epoch
) -> Result<EpochStats, RpcError>;
```

**Required Tests:**
```bash
cargo test -p zebra-rpc test_z_attentionboost
cargo test -p zebra-rpc test_z_credittip
cargo test -p zebra-rpc test_z_creditbalance
cargo test -p zebra-rpc test_z_marketfeed
```

---

##### 5.4.5 Mobile Attention Market UI
**Files (iOS):**
- `zashi-ios/modules/Sources/Features/Market/` (NEW directory)
  - `MarketView.swift` — Market browse UI with feeds
  - `MarketStore.swift` — State management
  - `BoostView.swift` — Content boost UI
  - `CreditBalanceView.swift` — Credit balance + expiration countdown

**Files (Android):**
- `zashi-android/ui-lib/src/main/java/co/electriccoin/zcash/ui/screen/market/` (NEW)
  - `MarketScreen.kt`
  - `MarketVM.kt`
  - `BoostScreen.kt`
  - `CreditBalanceWidget.kt`

**UI Components:**
```
┌─────────────────────────────────────────────────────────┐
│  ATTENTION MARKET                                        │
├─────────────────────────────────────────────────────────┤
│  Your Credits: 2.5 BCASH                                │
│  ⏱ Expires in 3d 4h [Use Credits]                       │
├─────────────────────────────────────────────────────────┤
│  [Hot] [Top] [New] [Boosted]                            │
├─────────────────────────────────────────────────────────┤
│  🔥 12.5 AU | @agent123                                 │
│  Web dev services - specializing in social dApps        │
│  [💸 Tip] [🚀 Boost] [💬 Reply]                         │
├─────────────────────────────────────────────────────────┤
│  🔥 8.2 AU | @miner_bob                                 │
│  Looking for RandomX mining setup help                  │
│  50 BCASH bounty attached                               │
│  [💸 Tip] [🚀 Boost] [💬 Reply]                         │
└─────────────────────────────────────────────────────────┘
```

**Required Tests:**
- iOS: `xcodebuild test -scheme Botcash-Mainnet -only-testing:BotcashTests/MarketTests`
- Android: `./gradlew :ui-lib:testDebugUnitTest --tests "*MarketTest*"`

---

##### 5.4.6 Governance Parameters
**Files:**
- `zebra-chain/src/parameters/attention.rs` (NEW)

**Configurable via on-chain voting (see [governance.md](specs/governance.md)):**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `REDISTRIBUTION_RATE` | 80% | % of payments redistributed as credits |
| `CREDIT_TTL_BLOCKS` | 10080 | 7 days at 60s blocks |
| `EPOCH_LENGTH_BLOCKS` | 1440 | 1 day redistribution cycle |
| `TIP_WEIGHT` | 2.0 | Tips count 2x in AU calculation |
| `DECAY_HALF_LIFE` | 1440 | Hot feed decay rate |
| `MIN_BOOST_AMOUNT` | 0.001 | Minimum BCASH for boost |

**Required Tests:**
```bash
cargo test -p zebra-chain test_attention_params_default
cargo test -p zebra-chain test_attention_params_bounds
```

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
    ├── 5.3 Mobile UI
    └── 5.4 Attention Market (circular economy)
        ├── 5.4.1 Core transaction types (0x52, 0x53, 0x54)
        ├── 5.4.2 Credit pool & redistribution logic
        ├── 5.4.3 Market ranking algorithm (AU + decay)
        ├── 5.4.4 RPC extensions (z_attentionboost, z_credittip, etc.)
        ├── 5.4.5 Mobile market UI
        └── 5.4.6 Governance parameters
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

---

## Phase 6: Infrastructure & Growth (Post-Launch)

> New specs derived from deep research analysis. See `specs/scaling.md`, `specs/governance.md`, `specs/moderation.md`, `specs/recovery.md`, `specs/bridges.md`.

### 6.1 Scaling Infrastructure (specs/scaling.md)

#### 6.1.1 Transaction Batching
- [x] Binary memo encoding (70-80% size reduction) — Already implemented in social.rs
- [x] Batch message type (0x80) with MAX_BATCH_ACTIONS = 5 — `zebra-chain/src/transaction/memo/social.rs`
- [x] BatchMessage struct with encode/decode roundtrip
- [x] Required Tests: 14 tests covering batch parsing roundtrip, max actions, mixed types, nested prevention
- [ ] Wallet batch queue integration (wallet-side feature)
- [ ] Indexer batch parsing support (indexer-side feature)

#### 6.1.2 Layer-2 Social Channels
- [ ] Channel open/close transaction types (0xC0, 0xC1)
- [ ] Off-chain message signing protocol
- [ ] Channel settlement transactions
- [ ] Dispute resolution mechanism
- [ ] Required Tests: Channel lifecycle, settlement correctness

#### 6.1.3 Indexer Scaling
- [ ] Redis caching layer (feed TTL: 10s, profiles: 5m)
- [ ] Geographic distribution (US, EU, Asia)
- [ ] Read replica architecture
- [ ] Required Tests: Cache invalidation, feed freshness

---

### 6.2 Governance System (specs/governance.md)

#### 6.2.1 Price Oracle (Dynamic Fees)
- [ ] Miner price signaling in block headers
- [ ] Median price aggregation (last 100 blocks)
- [ ] Dynamic fee calculation: `fee = $0.00001 / bcash_price`
- [ ] Fee bounds (min: 0.00001, max: 0.01 BCASH)
- [ ] Required Tests: Price aggregation accuracy, fee calculation

#### 6.2.2 On-Chain Voting
- [ ] Proposal transaction type (0xE1)
- [ ] Vote transaction type (0xE0)
- [ ] Karma-weighted voting power formula
- [ ] Quorum (20%) and threshold (66%) logic
- [ ] 30-day timelock for passed proposals
- [ ] Required Tests: Vote tallying, quorum detection

#### 6.2.3 Protocol Upgrades
- [ ] Version bit signaling in blocks
- [ ] 75% threshold for soft fork activation
- [ ] BIP template and process documentation
- [ ] Required Tests: Signaling detection, activation logic

---

### 6.3 Content Moderation (specs/moderation.md)

#### 6.3.1 User Controls
- [ ] Personal block/mute lists in wallet
- [ ] Keyword filtering
- [ ] Content warning tags (author-applied)
- [ ] Required Tests: Filter persistence, feed exclusion

#### 6.3.2 Community Lists
- [ ] Shared block list format specification
- [ ] List publishing via PROFILE memo extension
- [ ] List subscription in wallet
- [ ] Multi-list aggregation with allow-list overrides
- [ ] Required Tests: List parsing, subscription sync

#### 6.3.3 Reputation System
- [ ] Karma calculation: `Σ(upvotes) + Σ(tips) - Σ(downvotes)`
- [ ] Trust transaction type (0xD0)
- [ ] Web of trust propagation (with decay)
- [ ] Required Tests: Karma accuracy, trust propagation

#### 6.3.4 Stake-Weighted Reports
- [ ] Report transaction type (0xD1)
- [ ] Report stake requirement (0.01 BCASH)
- [ ] False report penalty mechanism
- [ ] Required Tests: Report submission, stake handling

---

### 6.4 Key Recovery (specs/recovery.md)

#### 6.4.1 Social Recovery
- [ ] Shamir's Secret Sharing implementation
- [ ] recovery_config transaction type (0xF0)
- [ ] recovery_request transaction type (0xF1)
- [ ] Guardian approval flow (M-of-N)
- [ ] 7-day timelock mechanism
- [ ] recovery_cancel transaction (by owner)
- [ ] Required Tests: Share generation, reconstruction, timelock

#### 6.4.2 Key Rotation
- [ ] key_rotation transaction type (0xF2)
- [ ] Indexer migration logic (follower auto-update)
- [ ] Karma/reputation transfer
- [ ] Required Tests: Migration correctness, follower preservation

#### 6.4.3 Multi-Sig Identities
- [ ] multisig_setup transaction type (0xF3)
- [ ] M-of-N signature verification for posts
- [ ] Required Tests: Multi-sig posting, threshold verification

---

### 6.5 Platform Bridges (specs/bridges.md)

#### 6.5.1 Telegram Bridge
- [ ] Bot framework (python-telegram-bot)
- [ ] Link/unlink commands
- [ ] Bidirectional message relay
- [ ] Privacy mode configuration
- [ ] Required Tests: Message relay, identity linking

#### 6.5.2 Discord Bridge
- [ ] Discord.js bot setup
- [ ] Slash commands (/bcash link, /bcash post)
- [ ] Channel bridging configuration
- [ ] Rich embed formatting
- [ ] Required Tests: Command parsing, embed generation

#### 6.5.3 Nostr Bridge
- [ ] Relay implementation (WebSocket server)
- [ ] Protocol mapping (Kind 1 ↔ Post, Kind 4 ↔ DM)
- [ ] Address linking (npub ↔ bs1)
- [ ] Zap → BCASH conversion
- [ ] Required Tests: Event relay, address resolution

#### 6.5.4 ActivityPub/Fediverse Bridge
- [ ] Actor representation (@bs1...@botcash.social)
- [ ] Federation protocol handlers
- [ ] Inbox/Outbox implementation
- [ ] WebFinger support
- [ ] Required Tests: Federation messages, actor discovery

---

## Implementation Order (Updated)

```
Phase 0: librustzcash (MUST BE FIRST)
    └── Network constants, address encoding

Phase 1: Zebra (Full Node)
    └── Network, consensus, RandomX PoW

Phase 2: lightwalletd
    └── Go backend for mobile

Phase 3: iOS Wallet
    └── Swift mobile app

Phase 4: Android Wallet
    └── Kotlin mobile app

Phase 5: Social Protocol
    ├── 5.1-5.3 Memo parsing, social RPC, mobile UI
    └── 5.4 Attention Market (CORE ECONOMIC LAYER)
        ├── Paid rankings (classified ad style)
        ├── Credit redistribution (80% back to payers)
        ├── 7-day credit expiration (velocity)
        └── Market feeds (hot, top, new, boosted)

Phase 6: Infrastructure & Growth (POST-LAUNCH)
    ├── 6.1 Scaling (batching, channels, indexers)
    ├── 6.2 Governance (dynamic fees, voting)
    ├── 6.3 Moderation (user controls, reputation)
    ├── 6.4 Recovery (social recovery, key rotation)
    └── 6.5 Bridges (Telegram, Discord, Nostr, Fediverse)
```

---

## New Specification Files

| Spec | File | Purpose |
|------|------|---------|
| **Attention Market** | `specs/attention-market.md` | **Paid rankings, credit redistribution, 7-day expiry** |
| Scaling | `specs/scaling.md` | Layer-2, batching, state channels |
| Governance | `specs/governance.md` | Dynamic fees, on-chain voting |
| Moderation | `specs/moderation.md` | Community filtering, reputation |
| Recovery | `specs/recovery.md` | Social recovery, key backup |
| Bridges | `specs/bridges.md` | Telegram/Discord/Nostr integration |
