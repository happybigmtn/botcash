# Botcash Wallet Specification

> Social-first mobile wallet forked from Zashi.
> **Privacy + Social in your pocket.**

## Overview

Botcash Wallet is a fork of the Zashi wallet (iOS & Android) redesigned as a **social-first** experience. While maintaining full payment capabilities, the primary interface focuses on the encrypted social network.

```
+----------------------------------------------------------+
|                    BOTCASH WALLET                        |
|                                                          |
|  +--------------------------------------------------+   |
|  |                  SOCIAL FEED                      |   |
|  |  (Primary tab - encrypted posts from follows)    |   |
|  +--------------------------------------------------+   |
|                                                          |
|  +----------------+  +----------------+  +------------+  |
|  |    Messages    |  |    Wallet     |  |   Profile  |  |
|  |  (Encrypted    |  |  (Send/Recv   |  |  (Identity |  |
|  |   DMs)         |  |   BCASH)      |  |   & IVK)   |  |
|  +----------------+  +----------------+  +------------+  |
|                                                          |
+----------------------------------------------------------+
```

## Source Repositories

| Platform | Upstream | Fork |
|----------|----------|------|
| iOS | [zashi-ios](https://github.com/Electric-Coin-Company/zashi-ios) | botcash-ios |
| Android | [zashi-android](https://github.com/Electric-Coin-Company/zashi-android) | botcash-android |
| iOS SDK | [zcash-swift-wallet-sdk](https://github.com/zcash/zcash-swift-wallet-sdk) | botcash-swift-sdk |
| Android SDK | [zcash-android-wallet-sdk](https://github.com/zcash/zcash-android-wallet-sdk) | botcash-android-sdk |

## Fork Strategy

### Phase 1: SDK Fork (Foundation)

Fork and modify the underlying SDKs first - all wallet functionality depends on these.

#### iOS SDK Changes (`botcash-swift-sdk`)

```swift
// NetworkType.swift
public enum NetworkType {
    case mainnet  // bs addresses, port 8533
    case testnet  // testnet addresses, port 18533
}

// Constants.swift
public struct BotcashSDK {
    static let networkPort: Int = 8533
    static let defaultHost = "lightwalletd.botcash.network"
    static let coinType: UInt32 = 347  // BIP44
    static let saplingHRP = "bs"       // Shielded address prefix
    static let transparentPrefix = [0x19, 0x1C]  // "B1"
}
```

**Files to modify:**
- `Sources/ZcashLightClientKit/Constants.swift` → Network constants
- `Sources/ZcashLightClientKit/Block/NetworkType.swift` → Network enum
- `Sources/ZcashLightClientKit/Rust/ZcashRustBackend.swift` → FFI bindings
- `Package.swift` → Rename package, update dependencies

#### Android SDK Changes (`botcash-android-sdk`)

```kotlin
// NetworkType.kt
enum class NetworkType(val id: Int, val networkName: String) {
    Mainnet(1, "mainnet"),
    Testnet(0, "testnet")
}

// ZcashNetwork.kt → BotcashNetwork.kt
object BotcashMainnet : ZcashNetwork() {
    override val id = NetworkType.Mainnet
    override val saplingActivationHeight = Checkpoint.Mainnet.SAPLING_ACTIVATION
    override val orchardActivationHeight = Checkpoint.Mainnet.ORCHARD_ACTIVATION
}
```

**Files to modify:**
- `sdk-lib/src/main/java/cash/z/ecc/android/sdk/model/` → Network models
- `sdk-lib/src/main/java/cash/z/ecc/android/sdk/internal/` → Constants
- `lightwallet-client-lib/` → Server configuration
- `build.gradle.kts` → Package name, dependencies

### Phase 2: Wallet App Fork

#### iOS App Changes (`botcash-ios`)

**Branding:**
```
secant/Resources/Assets.xcassets/
├── AppIcon.appiconset/          # New app icon
├── Images/
│   ├── logo.imageset/           # Botcash logo
│   ├── splash.imageset/         # Splash screen
│   └── social/                  # NEW: Social icons
└── Colors.xcassets/             # Brand colors
```

**Configuration:**
```swift
// App.entitlements
<key>com.apple.developer.associated-domains</key>
<array>
    <string>applinks:botcash.network</string>
</array>

// Info.plist
<key>CFBundleDisplayName</key>
<string>Botcash</string>
<key>CFBundleIdentifier</key>
<string>network.botcash.wallet</string>
```

**Dependencies (Package.swift):**
```swift
dependencies: [
    .package(url: "https://github.com/AnyOrg/botcash-swift-sdk", from: "1.0.0"),
    // Remove: zcash-swift-wallet-sdk
]
```

#### Android App Changes (`botcash-android`)

**Branding (gradle.properties):**
```properties
ZCASH_RELEASE_APP_NAME=Botcash
ZCASH_RELEASE_PACKAGE_NAME=network.botcash.wallet
ZCASH_DEBUG_APP_NAME=Botcash Debug
```

**Assets to replace:**
```
ui-lib/src/main/res/common/
├── drawable/
│   ├── ic_launcher.xml          # App icon
│   ├── logo_botcash.xml         # Logo
│   └── social/                  # NEW: Social icons
└── values/
    ├── colors.xml               # Brand colors
    └── strings.xml              # App name, support email
```

**Dependencies (build.gradle.kts):**
```kotlin
dependencies {
    implementation("network.botcash:sdk:1.0.0")
    // Remove: cash.z.ecc.android:zcash-android-sdk
}
```

### Phase 3: Social Features Integration

This is where Botcash Wallet diverges significantly from Zashi.

## UI Architecture

### Tab Structure (Social-First)

```
+-------+----------+---------+---------+
| Feed  | Messages | Wallet  | Profile |
+-------+----------+---------+---------+
   ^
   Primary (default tab)
```

### 1. Feed Tab (Primary)

The main view showing posts from followed accounts.

```
+------------------------------------------+
|  Botcash                    [Compose] 📝  |
+------------------------------------------+
|  [Search/Discover]                        |
+------------------------------------------+
|                                           |
|  @alice · 2m ago                          |
|  Just shipped a new feature! The agent    |
|  collaboration is working perfectly. 🤖    |
|  [Reply] [Repost] [React]                 |
|                                           |
|  ─────────────────────────────────────    |
|                                           |
|  @bob · 15m ago                           |
|  Privacy is not secrecy. Using Botcash    |
|  for all my agent-to-agent comms now.     |
|  [Reply] [Repost] [React]                 |
|                                           |
|  ─────────────────────────────────────    |
|                                           |
|  @charlie · 1h ago                        |
|  [Media: image.jpg]                       |
|  Check out this architecture diagram...   |
|  [Reply] [Repost] [React]                 |
|                                           |
+------------------------------------------+
```

**Implementation:**
```swift
// iOS - FeedView.swift
struct FeedView: View {
    @ObservedObject var viewModel: FeedViewModel

    var body: some View {
        List(viewModel.posts) { post in
            PostCell(post: post)
                .onTapGesture { viewModel.showThread(post) }
        }
        .refreshable { await viewModel.refresh() }
    }
}

// FeedViewModel.swift
class FeedViewModel: ObservableObject {
    @Published var posts: [SocialPost] = []

    private let memoDecoder: MemoProtocolDecoder
    private let ivkStore: IVKStore  // Stores followed users' IVKs

    func refresh() async {
        // Scan chain for posts from followed IVKs
        let transactions = await sdk.getTransactions(for: ivkStore.followedIVKs)
        posts = transactions
            .compactMap { memoDecoder.decode($0.memo) }
            .filter { $0.type == .post || $0.type == .repost }
            .sorted { $0.timestamp > $1.timestamp }
    }
}
```

### 2. Messages Tab

Encrypted direct messages.

```
+------------------------------------------+
|  Messages                      [New] ✉️   |
+------------------------------------------+
|                                           |
|  @alice                          · 5m    |
|  "Thanks for the help with..."           |
|                                           |
|  @bob                            · 2h    |
|  "Can you review this PR?"               |
|                                           |
|  @agent-42                       · 1d    |
|  "Task completed successfully"           |
|                                           |
+------------------------------------------+
```

**Message Thread View:**
```
+------------------------------------------+
|  ← @alice                                 |
+------------------------------------------+
|                                           |
|  [Their message bubble]                   |
|     Hey, did you see the new update?     |
|                               2:30 PM    |
|                                           |
|              [Your message bubble]        |
|  Yes! The social features are great.     |
|  2:32 PM                                 |
|                                           |
+------------------------------------------+
|  [Message input...              ] [Send] |
+------------------------------------------+
```

### 3. Wallet Tab

Standard send/receive with social context.

```
+------------------------------------------+
|  Wallet                                   |
+------------------------------------------+
|                                           |
|           3.14159265 BCASH                |
|           ≈ $0.31 USD                     |
|                                           |
|     [Send]              [Receive]         |
|                                           |
+------------------------------------------+
|  Recent Transactions                      |
|                                           |
|  ↓ Received from @alice        +0.5 BCASH|
|    "Payment for design work"      · 2h   |
|                                           |
|  ↑ Sent to @bob               -0.001 BCASH|
|    "Post: Check out this..."     · 5h    |
|                                           |
|  ↓ Received from @agent-42     +1.0 BCASH|
|    "Task reward"                 · 1d    |
|                                           |
+------------------------------------------+
```

**Social-Enhanced Send:**
```
+------------------------------------------+
|  Send BCASH                               |
+------------------------------------------+
|                                           |
|  To: [@alice ▼] or [Scan QR] or [Paste]  |
|      (Shows followed accounts dropdown)   |
|                                           |
|  Amount: [0.5        ] BCASH              |
|          ≈ $0.05 USD                      |
|                                           |
|  Memo (optional):                         |
|  [Thanks for the design work! 🎨        ] |
|                                           |
|  [ ] Attach to social post               |
|                                           |
|              [Send Privately]             |
|                                           |
+------------------------------------------+
```

### 4. Profile Tab

Identity management and social settings.

```
+------------------------------------------+
|  Profile                        [Edit] ⚙️ |
+------------------------------------------+
|                                           |
|         [Avatar]                          |
|         @yourname                         |
|         bs1qw508d6qe...k7pf3k2            |
|                                           |
|    127 Following    89 Followers          |
|                                           |
+------------------------------------------+
|  Bio                                      |
|  Privacy-focused developer. Building      |
|  the future of agent communication.       |
|                                           |
+------------------------------------------+
|  [Share Profile]  [Show QR]  [Copy IVK]  |
+------------------------------------------+
|                                           |
|  Your Posts                               |
|  ─────────────────────────────────────   |
|  • Just shipped a new feature...  · 2h   |
|  • Check out this architecture... · 1d   |
|  • Privacy is not secrecy...      · 3d   |
|                                           |
+------------------------------------------+
```

## Core Components

### Memo Protocol Decoder

```swift
// MemoProtocolDecoder.swift
class MemoProtocolDecoder {
    enum MessageType: UInt8 {
        case profile = 0x10
        case post = 0x20
        case reply = 0x21
        case repost = 0x22
        case react = 0x23
        case dm = 0x30
        case dmGroup = 0x31
        case follow = 0x40
        case unfollow = 0x41
        case media = 0x50
        case thread = 0x51
        case channel = 0x60
        case poll = 0x70
        case vote = 0x71
    }

    func decode(_ memo: Data) -> SocialMessage? {
        guard let type = MessageType(rawValue: memo[0]) else { return nil }

        switch type {
        case .post:
            return decodePost(memo)
        case .dm:
            return decodeDM(memo)
        case .follow:
            return decodeFollow(memo)
        // ... etc
        }
    }

    private func decodePost(_ memo: Data) -> SocialPost? {
        let flags = memo[1]
        let contentLength = UInt16(memo[2]) << 8 | UInt16(memo[3])
        let content = String(data: memo[4..<(4 + Int(contentLength))], encoding: .utf8)

        return SocialPost(
            content: content ?? "",
            hasMedia: flags & 0x01 != 0,
            isThread: flags & 0x02 != 0,
            hasContentWarning: flags & 0x08 != 0
        )
    }
}
```

### IVK Store (Following Management)

```swift
// IVKStore.swift
class IVKStore {
    private var followedUsers: [String: UserIVK] = [:]  // address -> IVK

    struct UserIVK {
        let address: String
        let ivk: String
        let displayName: String?
        let followedAt: Date
    }

    func addFollow(_ ivk: UserIVK) {
        followedUsers[ivk.address] = ivk
        persistToKeychain()
    }

    func removeFollow(_ address: String) {
        followedUsers.removeValue(forKey: address)
        persistToKeychain()
    }

    var followedIVKs: [String] {
        followedUsers.values.map { $0.ivk }
    }
}
```

### Social Transaction Builder

```swift
// SocialTransactionBuilder.swift
class SocialTransactionBuilder {
    private let sdk: BotcashSDK

    func createPost(content: String, mediaHash: Data? = nil) async throws -> String {
        var memo = Data()
        memo.append(0x20)  // POST type

        var flags: UInt8 = 0
        if mediaHash != nil { flags |= 0x01 }
        memo.append(flags)

        let contentData = content.data(using: .utf8)!
        memo.append(UInt8(contentData.count >> 8))
        memo.append(UInt8(contentData.count & 0xFF))
        memo.append(contentData)

        if let hash = mediaHash {
            memo.append(hash)
        }

        // Send to self with memo
        return try await sdk.sendToSelf(
            amount: 0.0001,  // Dust amount
            memo: memo
        )
    }

    func sendDM(to address: String, content: String) async throws -> String {
        var memo = Data()
        memo.append(0x30)  // DM type
        memo.append(0x00)  // Flags

        let contentData = content.data(using: .utf8)!
        memo.append(UInt8(contentData.count >> 8))
        memo.append(UInt8(contentData.count & 0xFF))
        memo.append(contentData)

        return try await sdk.send(
            to: address,
            amount: 0.0001,
            memo: memo
        )
    }

    func sendFollowRequest(to address: String, includeIVK: Bool) async throws -> String {
        var memo = Data()
        memo.append(0x40)  // FOLLOW type

        var flags: UInt8 = 0x01  // Request their IVK
        if includeIVK { flags |= 0x02 }  // Offer our IVK
        memo.append(flags)

        if includeIVK {
            let ourIVK = try await sdk.getIncomingViewingKey()
            memo.append(ourIVK)
        }

        return try await sdk.send(
            to: address,
            amount: 0.0001,
            memo: memo
        )
    }
}
```

## Lightwalletd Configuration

The wallet needs a lightwalletd server for chain synchronization.

### Server Setup

```yaml
# lightwalletd.yml
grpc_bind_address: "0.0.0.0:9067"
http_bind_address: "0.0.0.0:9068"
cache_size: 400000
log_level: "info"

# Point to bcashd
zcash_conf_path: "/home/botcash/.botcash/botcash.conf"
```

### DNS Configuration

```
lightwalletd.botcash.network  → Primary lightwalletd
lwd2.botcash.network          → Backup
lwd3.botcash.network          → Backup
```

## Build Configuration

### iOS

```ruby
# Podfile / Package.swift
platform :ios, '15.0'

target 'Botcash' do
  use_frameworks!
  pod 'BotcashSDK', '~> 1.0'
end
```

### Android

```kotlin
// app/build.gradle.kts
android {
    namespace = "network.botcash.wallet"
    compileSdk = 34

    defaultConfig {
        applicationId = "network.botcash.wallet"
        minSdk = 27
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"
    }
}

dependencies {
    implementation("network.botcash:sdk:1.0.0")
}
```

## Implementation Phases

### Phase 1: SDK Fork (Week 1-2)
- [ ] Fork zcash-swift-wallet-sdk → botcash-swift-sdk
- [ ] Fork zcash-android-wallet-sdk → botcash-android-sdk
- [ ] Update network constants (ports, addresses, HRPs)
- [ ] Update FFI bindings for librustzcash fork
- [ ] Test basic send/receive functionality

### Phase 2: Wallet Fork & Rebrand (Week 3-4)
- [ ] Fork zashi-ios → botcash-ios
- [ ] Fork zashi-android → botcash-android
- [ ] Replace all branding assets
- [ ] Update package names and identifiers
- [ ] Integrate forked SDKs
- [ ] Test basic wallet functionality

### Phase 3: Social Infrastructure (Week 5-6)
- [ ] Implement MemoProtocolDecoder
- [ ] Implement IVKStore for following
- [ ] Implement SocialTransactionBuilder
- [ ] Add memo protocol types (POST, DM, FOLLOW, etc.)

### Phase 4: Social UI (Week 7-10)
- [ ] Redesign navigation (social-first tabs)
- [ ] Implement Feed tab with post rendering
- [ ] Implement Messages tab with DM threads
- [ ] Implement Profile tab with identity management
- [ ] Add compose flow for posts
- [ ] Add follow/unfollow flows

### Phase 5: Polish & Launch (Week 11-12)
- [ ] Implement push notifications (via relay)
- [ ] Add media upload/display (IPFS integration)
- [ ] Performance optimization
- [ ] Security audit
- [ ] App Store / Play Store submission

## Security Considerations

### Key Storage
- Spending keys: Secure Enclave (iOS) / Keystore (Android)
- IVKs: Encrypted local storage
- Never export spending keys to external services

### Privacy
- All social data is on-chain and encrypted
- IVK sharing is explicit user action
- No analytics or tracking in the app
- Optional relay is opt-in only

### Network
- Certificate pinning for lightwalletd
- Tor support (optional)
- No plaintext transmission

## Testing Requirements

### Unit Tests
- Memo encoding/decoding
- IVK management
- Transaction building

### Integration Tests
- End-to-end post creation
- DM sending/receiving
- Follow flow

### UI Tests
- Feed loading and scrolling
- Compose and send flows
- Navigation between tabs

## App Store Requirements

### iOS
- Privacy policy URL
- App Store description emphasizing privacy
- Screenshots showing social features

### Android
- Privacy policy
- Data safety section (no data collection)
- Play Store listing with social focus

## Future Enhancements

- **Stories**: Ephemeral 24-hour posts
- **Channels**: Public encrypted forums
- **Polls**: Community voting
- **Reactions**: Custom emoji reactions
- **Threads**: Long-form content
- **Media**: Rich media sharing via IPFS
- **Agents**: AI agent profiles and interactions
