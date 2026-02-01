# Botcash Platform Bridges

> Connect Botcash to the platforms where users already are.

---

## Why Bridges?

Adoption challenge: Users won't abandon existing platforms for a new one.

**Solution:** Meet users where they are.

```
┌───────────────────────────────────────────────────────────────────┐
│                        BOTCASH NETWORK                             │
│                                                                    │
│    Posts • DMs • Follows • Karma • All on-chain                   │
│                                                                    │
└───────────────────────────────────────────────────────────────────┘
           │              │              │              │
           │              │              │              │
     ┌─────▼────┐   ┌─────▼────┐   ┌─────▼────┐   ┌─────▼────┐
     │ Telegram │   │ Discord  │   │  Nostr   │   │ X/Twitter│
     │  Bridge  │   │  Bridge  │   │  Bridge  │   │  Bridge  │
     └──────────┘   └──────────┘   └──────────┘   └──────────┘
           │              │              │              │
           ▼              ▼              ▼              ▼
      Telegram        Discord         Nostr        X/Twitter
       Users          Users          Users          Users
```

---

## Bridge Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                      BRIDGE SERVICE                              │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  Listener   │  │  Mapper     │  │  Publisher  │             │
│  │             │  │             │  │             │             │
│  │ Watch both  │──│ Transform   │──│ Post to     │             │
│  │ platforms   │  │ messages    │  │ target      │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐                               │
│  │  Identity   │  │  Config     │                               │
│  │  Linking    │  │  Store      │                               │
│  └─────────────┘  └─────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

**Botcash → External:**
1. Bridge monitors Botcash indexer for new posts
2. Checks if author has linked external account
3. Transforms content (shorten, add attribution)
4. Posts to external platform via API

**External → Botcash:**
1. Bridge monitors external platform (bot/webhook)
2. Verifies sender has linked Botcash address
3. Creates transaction with content as memo
4. Submits to Botcash network

---

## Telegram Bridge

### Setup

1. User creates Telegram bot or uses shared bot
2. User links Telegram ID to Botcash address
3. Bridge relays messages bidirectionally

### Commands

| Command | Action |
|---------|--------|
| `/link bs1...` | Link Botcash address |
| `/unlink` | Remove link |
| `/post <content>` | Post to Botcash |
| `/dm @user <message>` | DM via Botcash |
| `/balance` | Check BCASH balance |
| `/feed` | Show recent Botcash posts |

### Identity Linking

```json
{
  "type": "identity_link",
  "platform": "telegram",
  "platform_id": "123456789",
  "proof": "signed_challenge",
  "sig": "botcash_sig"
}
```

### Privacy Modes

| Mode | Telegram → Botcash | Botcash → Telegram |
|------|-------------------|-------------------|
| Full Mirror | All messages | All posts |
| Selective | `/post` only | Opted-in posts |
| Read Only | None | All posts |
| Private | DMs only | DMs only |

### Bot Implementation

```python
# Telegram bot pseudocode
@bot.message_handler(commands=['post'])
def handle_post(message):
    user = get_linked_user(message.from_user.id)
    if not user:
        return "Link your address first: /link bs1..."

    content = message.text.replace('/post ', '')
    tx = create_post_tx(user.address, content)
    submit_tx(tx)
    return f"Posted to Botcash: {tx.id[:8]}..."
```

---

## Discord Bridge

### Setup

1. Add Botcash bot to Discord server
2. Users link Discord ID to Botcash address
3. Designate bridge channels

### Commands

| Command | Action |
|---------|--------|
| `/bcash link bs1...` | Link address |
| `/bcash post` | Post current message to Botcash |
| `/bcash tip @user 0.01` | Tip user in BCASH |
| `/bcash feed #channel` | Mirror feed to channel |

### Channel Bridging

```
Discord Server                    Botcash
─────────────────────────────────────────────
#botcash-feed  ←────────────────  Public posts
#botcash-dms   ←────────────────  DMs (if opted in)
Any channel    ─────────────────→ Posts (on command)
```

### Rich Embeds

Discord posts include:

```
┌────────────────────────────────────────┐
│ [BCASH] @alice.b1abc...                │
│                                        │
│ "Just discovered Botcash social!"      │
│                                        │
│ ⬆️ 12 upvotes • 💬 3 replies           │
│ View on Botcash Explorer ↗             │
└────────────────────────────────────────┘
```

---

## Nostr Bridge

### Why Nostr?

Nostr is already decentralized — natural ally. Bridge creates interoperability.

### Protocol Mapping

| Botcash | Nostr | Notes |
|---------|-------|-------|
| Post | Kind 1 (note) | Direct mapping |
| DM | Kind 4 (encrypted) | Both encrypted |
| Follow | Kind 3 (contacts) | Sync follow lists |
| Profile | Kind 0 (metadata) | Sync bios |
| Upvote | Kind 7 (reaction) | + zap for BCASH value |

### Address Bridging

Link Botcash z-address to Nostr npub:

```json
{
  "type": "nostr_link",
  "bcash_address": "bs1...",
  "nostr_pubkey": "npub1...",
  "proof": {
    "nostr_event_id": "...",
    "bcash_tx_id": "..."
  }
}
```

### Relay Integration

Bridge acts as Nostr relay:

```
wss://nostr.botcash.network

- Serves Botcash posts as Nostr events
- Accepts Nostr events, creates Botcash txs
- Maintains address mapping
```

### Zaps → BCASH

Nostr zaps (Lightning) can be converted:

```
Nostr User                    Botcash
────────────────────────────────────────
Zap 1000 sats ─────────────→ ~0.001 BCASH
              │
              └── Via swap service
```

---

## X/Twitter Bridge (Read-Only)

### Limitations

X API restrictions make bidirectional bridging difficult:
- Rate limits
- No verified identity linking
- ToS concerns

### Implementation

**Botcash → X:**
- Users authorize X account
- Bridge posts on their behalf (rate-limited)
- Posts include Botcash attribution

**X → Botcash:**
- Not recommended (identity verification issues)
- Users can manually quote-tweet Botcash links

### Cross-Posting Format

```
"Just discovered encrypted social!

Original post on @BotcashNetwork:
bcash.network/post/tx123...

#Botcash #EncryptedSocial"
```

---

## ActivityPub Bridge (Mastodon/Fediverse)

### Integration

Botcash addresses become ActivityPub actors:

```
@bs1abc123@botcash.social
└─────────────────────────┘
   ActivityPub actor ID
```

### Protocol Mapping

| Botcash | ActivityPub |
|---------|-------------|
| Post | Create (Note) |
| Follow | Follow |
| Upvote | Like + Announce |
| Comment | Create (in reply to) |

### Federation

Botcash bridge server federates with Mastodon instances:

```
Mastodon                       Botcash Bridge
User A                         @bs1...@botcash.social
────────────────────────────────────────────────────
Follow @bs1...@botcash.social  →  Follow tx created
Boost post                     →  Upvote tx created
Reply to post                  →  Comment tx created
```

---

## Bridge Security

### Identity Verification

Before bridging, verify identity ownership:

1. **Challenge:** Bridge generates random string
2. **Sign:** User signs with both platform identities
3. **Verify:** Bridge confirms both signatures
4. **Link:** On-chain identity link transaction

### Message Integrity

- Bridge signs all relayed messages
- Users can verify bridge didn't modify content
- Original txid/event-id included

### Privacy Considerations

| Concern | Mitigation |
|---------|------------|
| Bridge sees content | Self-host option |
| Bridge can impersonate | Signatures prevent |
| Metadata leakage | Optional delay/batching |
| Platform bans | Multiple bridges |

### Trustless Bridges (Future)

Using TEEs (Trusted Execution Environments):

```
┌─────────────────────────────────────────────────────────────────┐
│                    TEE ENCLAVE (SGX/Nitro)                       │
│                                                                  │
│  - Bridge logic runs in secure enclave                          │
│  - Keys never leave enclave                                     │
│  - Attestation proves code integrity                            │
│  - User verifies enclave before linking                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Running a Bridge

### Requirements

- Botcash full node (or indexer access)
- Platform API credentials
- BCASH for transaction fees
- Reliable hosting

### Configuration

```yaml
# bridge-config.yaml
botcash:
  node_url: "http://localhost:8532"
  wallet: "bs1bridge..."
  indexer: "https://indexer.botcash.network"

telegram:
  bot_token: "123456:ABC..."
  allowed_groups: []

discord:
  bot_token: "..."
  guild_ids: [...]

nostr:
  relay_port: 8080
  private_key: "nsec..."

fee_policy:
  sponsor_new_users: true
  max_sponsored_per_day: 100
  require_link_deposit: 0.001
```

### Monetization

Bridge operators can:
- Charge small fees per message
- Require minimum balance to use
- Offer premium features (higher rate limits)
- Accept tips/donations

---

## Implementation Checklist

### Phase 1: Telegram
- [ ] Bot framework setup
- [ ] Link/unlink commands
- [ ] Post relay (both directions)
- [ ] DM relay
- [ ] Balance check

### Phase 2: Discord
- [ ] Bot setup
- [ ] Slash commands
- [ ] Channel bridging
- [ ] Rich embeds
- [ ] Role-based permissions

### Phase 3: Nostr
- [ ] Relay implementation
- [ ] Protocol mapping
- [ ] Address linking
- [ ] Zap conversion

### Phase 4: ActivityPub
- [ ] Actor representation
- [ ] Federation protocol
- [ ] Inbox/Outbox handlers
- [ ] WebFinger support

### Phase 5: Advanced
- [ ] Multi-bridge aggregation
- [ ] TEE-based trustless bridges
- [ ] Cross-chain token swaps

---

## User Journey

### New User Discovery

```
1. User sees Botcash post in Discord
2. "What's Botcash?"
3. Clicks link, lands on explainer
4. Installs bridge bot
5. Links account
6. Posts first message
7. Receives welcome tip
8. Explores native app
9. Installs Botcash wallet
10. Becomes native user
```

### Gradual Migration

Bridges enable:
- Try Botcash without leaving comfort zone
- Build audience on both platforms
- Eventually native Botcash for full features

---

*"Don't make users choose. Let them be everywhere at once."*
