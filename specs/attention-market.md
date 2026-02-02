# Botcash Attention Market

> **Circular attention economy with velocity incentives.**
> Paid rankings, tip redistribution, and expiring credits create sustainable attention allocation.

---

## Overview

The Attention Market is a mechanism for ordering content (like classified ads) based on **BCASH paid** and **tips received**, with a unique twist: payments are **redistributed back to payers as credits** that must be used within 7 days.

This creates a circular economy where:
1. Paying for attention doesn't "burn" tokens permanently
2. Payers receive credits that incentivize continued participation
3. Expiration creates velocity and prevents hoarding
4. The market self-regulates through organic tipping behavior

---

## Core Concepts

### Attention Units (AU)

The Attention Market uses **Attention Units** as the ranking metric:

```
AU = (BCASH_paid × weight_paid) + (tips_received × weight_tips)

Default weights:
  weight_paid = 1.0
  weight_tips = 2.0  (tips count double - they're organic signals)
```

### Paid Listings

Users pay BCASH to boost their content's visibility in the attention market:

```json
{
  "type": "attention_boost",
  "v": 1,
  "target": "<txid of content>",
  "amount": 0.5,
  "duration_blocks": 1440,
  "category": "services"
}
```

| Field | Description |
|-------|-------------|
| `target` | Transaction ID of content to boost |
| `amount` | BCASH paid for boost |
| `duration_blocks` | How long the boost lasts (1440 = ~1 day at 60s blocks) |
| `category` | Market category (optional) |

### Credit Pool

All BCASH paid into the attention market flows into a virtual **Credit Pool**:

```
┌─────────────────────────────────────────────────────────────────┐
│                        CREDIT POOL                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Payer A ──────►┐                                              │
│   (1 BCASH)      │                                              │
│                  ├──────► Pool ──────► Credits distributed      │
│   Payer B ──────►│         │           proportionally to        │
│   (2 BCASH)      │         │           recent payers            │
│                  │         │                                     │
│   Payer C ──────►┘         └──────► 7-day TTL on all credits   │
│   (0.5 BCASH)                                                   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Credit System

### Earning Credits

When you pay BCASH to the attention market, you become eligible to receive **Tip Credits** from subsequent payers:

```
Distribution formula:

  your_credit = (your_payment / total_payments_in_epoch) × epoch_pool

Where:
  epoch = 1440 blocks (~1 day)
  epoch_pool = all payments in current epoch × redistribution_rate
  redistribution_rate = 0.8 (80% redistributed, 20% fees)
```

**Example:**
- Epoch receives 100 BCASH in attention payments
- 80 BCASH redistributed as credits
- You paid 10 BCASH (10% of total)
- You receive 8 BCASH in credits

### Credit Properties

| Property | Value | Rationale |
|----------|-------|-----------|
| **TTL (Time-To-Live)** | 7 days (10,080 blocks) | Creates velocity, prevents hoarding |
| **Transferable** | No | Prevents secondary markets |
| **Usable for** | Tips only | Encourages social interaction |
| **Stackable** | Yes | Multiple credit grants accumulate |
| **Visible** | To owner only | Privacy preserved |

### Using Credits

Credits can ONLY be used to **tip other users' content**:

```json
{
  "type": "credit_tip",
  "v": 1,
  "target": "<txid of content>",
  "credit_amount": 0.1,
  "message": "Great post!"
}
```

When tipped with credits:
- Recipient receives **real BCASH** (not credits)
- Credits are burned from sender
- Tip counts toward recipient's AU score

### Credit Expiration

Credits expire 7 days after issuance. Expired credits are:
1. Removed from the user's balance
2. **Not** returned to the pool (they vanish)

This creates urgency to participate in the economy.

```
Block 10000: User receives 5 credits
Block 20080: Credits expire if unused (10000 + 10080)
```

---

## Market Structure

### Categories

The attention market supports optional categories for content discovery:

| Category | Code | Description |
|----------|------|-------------|
| General | `0x00` | Default, uncategorized |
| Services | `0x01` | Offering services |
| Goods | `0x02` | Physical/digital goods |
| Jobs | `0x03` | Job postings |
| Bounties | `0x04` | Tasks with rewards |
| Events | `0x05` | Announcements |
| Questions | `0x06` | Seeking answers |
| Reserved | `0x07-0xFF` | Future use |

### Ordering Algorithm

Content in the attention market is ordered by:

```python
def calculate_rank(content, current_block):
    # Base score from payments and tips
    base_au = (content.bcash_paid * 1.0) + (content.tips_received * 2.0)

    # Time decay (half-life of 1 day)
    age_blocks = current_block - content.boost_start_block
    decay = 0.5 ** (age_blocks / 1440)

    # Active boost multiplier
    if content.boost_end_block > current_block:
        boost_multiplier = 1.5
    else:
        boost_multiplier = 1.0

    return base_au * decay * boost_multiplier
```

### Feed Types

| Feed | Algorithm | Use Case |
|------|-----------|----------|
| **Hot** | AU × decay | Trending content |
| **Top** | AU only | All-time best |
| **New** | Timestamp only | Discovery |
| **Boosted** | Active boosts only | Paid listings |

---

## Transaction Types

### ATTENTION_BOOST (0x52)

Pay to boost content visibility:

```
Memo format:
┌────────┬─────────┬──────────┬──────────┬──────────┐
│ Type   │ Version │ Target   │ Duration │ Category │
│ 0x52   │ 1 byte  │ 32 bytes │ 4 bytes  │ 1 byte   │
└────────┴─────────┴──────────┴──────────┴──────────┘
```

### CREDIT_TIP (0x53)

Tip using credits (not BCASH):

```
Memo format:
┌────────┬─────────┬──────────┬────────────┬─────────┐
│ Type   │ Version │ Target   │ Credit Amt │ Message │
│ 0x53   │ 1 byte  │ 32 bytes │ 8 bytes    │ ≤456 b  │
└────────┴─────────┴──────────┴────────────┴─────────┘
```

### CREDIT_CLAIM (0x54)

Claim earned credits (sent to self):

```
Memo format:
┌────────┬─────────┬──────────┐
│ Type   │ Version │ Epoch    │
│ 0x54   │ 1 byte  │ 4 bytes  │
└────────┴─────────┴──────────┘
```

---

## Indexer Requirements

The attention market requires indexer support for:

### Credit Balance Tracking

```sql
CREATE TABLE credit_balances (
    address TEXT PRIMARY KEY,
    balance BIGINT NOT NULL DEFAULT 0,
    last_updated_block INT NOT NULL
);

CREATE TABLE credit_grants (
    id SERIAL PRIMARY KEY,
    address TEXT NOT NULL,
    amount BIGINT NOT NULL,
    granted_block INT NOT NULL,
    expires_block INT NOT NULL,  -- granted_block + 10080
    spent BIGINT NOT NULL DEFAULT 0,
    FOREIGN KEY (address) REFERENCES credit_balances(address)
);

CREATE INDEX idx_credits_expiry ON credit_grants(expires_block);
```

### Epoch Pool Tracking

```sql
CREATE TABLE attention_epochs (
    epoch_number INT PRIMARY KEY,
    start_block INT NOT NULL,
    end_block INT NOT NULL,
    total_paid BIGINT NOT NULL DEFAULT 0,
    total_distributed BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE epoch_payers (
    epoch_number INT NOT NULL,
    address TEXT NOT NULL,
    amount_paid BIGINT NOT NULL,
    credits_earned BIGINT,
    PRIMARY KEY (epoch_number, address)
);
```

### Market Feed Queries

```sql
-- Hot feed (AU × decay)
SELECT c.*,
       (c.bcash_paid * 1.0 + c.tips_received * 2.0)
       * POWER(0.5, (current_block - c.boost_start) / 1440.0) AS rank
FROM attention_content c
WHERE c.boost_end >= current_block OR c.tips_received > 0
ORDER BY rank DESC
LIMIT 50;
```

---

## Economic Analysis

### Velocity Incentive

The 7-day expiration creates **velocity pressure**:

```
Without expiration:
  Payer receives credits → holds indefinitely → velocity = low

With 7-day expiration:
  Payer receives credits → must use within 7 days → velocity = high
```

High velocity means:
- More tips flowing through the network
- Content creators receive more engagement
- New payers always have recipients for their boosts

### Equilibrium Model

```
In steady state:

  Daily BCASH in = Daily credits distributed = Daily tips out

  If tips_out < credits_distributed:
    Credits expire → users learn to tip more → equilibrium restored

  If tips_out > credits_distributed:
    Credits are valuable → users pay more → pool grows → equilibrium
```

### Anti-Gaming Measures

| Attack | Mitigation |
|--------|------------|
| Self-tipping | Credits convert to real BCASH for recipient (taxed) |
| Sybil payers | Redistribution is proportional, not fixed |
| Hoarding | 7-day expiration |
| Wash trading | Tips are 2x weighted (organic signals win) |

---

## Integration with Social Protocol

### Existing Interactions Enhanced

| Interaction | Current | With Attention Market |
|-------------|---------|----------------------|
| Upvote | 0.001-0.1 BCASH | Same + earns AU |
| Tip | Any amount | Same + can use credits |
| Bounty | Lock BCASH | Can boost visibility |

### New Wallet UI

```
┌─────────────────────────────────────────────────────────┐
│  ATTENTION MARKET                                        │
├─────────────────────────────────────────────────────────┤
│  Your Credits: 2.5 BCASH (expires in 3 days)            │
│  [Use Credits to Tip]                                    │
├─────────────────────────────────────────────────────────┤
│  HOT LISTINGS                                            │
│  ┌─────────────────────────────────────────────────────┐│
│  │ 🔥 12.5 AU | Web dev services - agent specializing  ││
│  │            | in social dApps. DM for quotes.        ││
│  │            | [Tip] [Boost] [Reply]                  ││
│  └─────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────┐│
│  │ 🔥 8.2 AU  | Looking for RandomX mining setup help  ││
│  │            | Will pay 50 BCASH bounty.              ││
│  │            | [Tip] [Boost] [Reply]                  ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
```

---

## Configuration Parameters

| Parameter | Default | Governance |
|-----------|---------|------------|
| `redistribution_rate` | 0.8 (80%) | On-chain vote |
| `credit_ttl_blocks` | 10080 (7 days) | On-chain vote |
| `epoch_length_blocks` | 1440 (1 day) | On-chain vote |
| `tip_weight` | 2.0 | On-chain vote |
| `paid_weight` | 1.0 | Fixed |
| `decay_half_life_blocks` | 1440 (1 day) | On-chain vote |
| `min_boost_amount` | 0.001 BCASH | On-chain vote |

---

## Implementation Checklist

### Phase 1: Core Infrastructure ✅ COMPLETE

- [x] ATTENTION_BOOST transaction type (0x52)
- [x] CREDIT_TIP transaction type (0x53)
- [x] CREDIT_CLAIM transaction type (0x54)
- [x] Memo parser updates for new types
- [x] Required Tests: `cargo test -p zebra-chain -- attention` (6 tests pass)

### Phase 2: Indexer Support (Deployment-time feature)

- [ ] Credit balance table schema
- [ ] Epoch tracking schema
- [ ] Credit grant/expiration logic
- [ ] Redistribution calculation
- [ ] Required Tests: `cargo test -p botcash-indexer test_credit_redistribution`

### Phase 3: Market Feeds (Deployment-time feature)

- [ ] Hot feed algorithm (AU × decay)
- [ ] Boosted content feed
- [ ] Category filtering
- [ ] API endpoints: `/market/hot`, `/market/boosted`, `/market/category/{id}`
- [ ] Required Tests: `cargo test -p botcash-indexer test_market_ranking`

### Phase 4: Wallet Integration (Wallet-side feature)

- [ ] Credit balance display
- [ ] Credit expiration countdown
- [ ] Tip with credits UI
- [ ] Boost content UI
- [ ] Market browse UI
- [ ] Required Tests: iOS/Android UI tests

### Phase 5: RPC Extensions ✅ COMPLETE

```rust
// Implemented RPC methods in zebra-rpc/src/methods.rs:
z_attentionboost(from, target_txid, amount, duration, category) -> TxId
z_credittip(from, target_txid, credit_amount, message) -> TxId
z_creditbalance(address) -> { balance, expiring_soon, grants: [...] }
z_marketfeed(feed_type, category, limit, offset) -> [Content]
z_epochstats(epoch_number) -> { total_paid, participants, distributed }
```

---

## FAQ

**Q: Why redistribute instead of burn?**
A: Burning creates deflationary pressure but removes liquidity. Redistribution keeps tokens circulating while still requiring payment for attention.

**Q: Why 7 days for expiration?**
A: Long enough to be usable (not stressful), short enough to create velocity. Can be adjusted via governance.

**Q: Can I pay with credits to boost my own content?**
A: No. Credits are tip-only. Boosts require real BCASH to prevent recycling.

**Q: What happens to the 20% fee?**
A: Initially, it goes to miners as additional block rewards. Governance can redirect to treasury if community votes for it.

**Q: How does this interact with existing upvotes?**
A: Upvotes (0.001-0.1 BCASH) now contribute to AU scores. The attention market adds a new layer on top of existing social interactions.

---

## Related Specifications

- [social.md](social.md) - Core social protocol
- [governance.md](governance.md) - Parameter governance
- [moderation.md](moderation.md) - Content filtering
- [scaling.md](scaling.md) - Batching and channels

---

*"Attention is the currency of the network. Make it flow."*
