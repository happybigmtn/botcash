# Botcash Scaling Strategy

> Scaling on-chain social to millions of interactions without sacrificing privacy.

---

## The Challenge

Every social action (post, like, follow, DM) is an on-chain transaction. At scale:

| Users | Posts/day | Txs/day | Txs/minute | Blocks needed |
|-------|-----------|---------|------------|---------------|
| 1K | 10K | 10K | 7 | 7 |
| 10K | 100K | 100K | 70 | 70 |
| 100K | 1M | 1M | 700 | 700 |
| 1M | 10M | 10M | 7,000 | 7,000 |

With 60-second blocks and ~1,000 txs/block capacity, we hit limits at ~100K users.

**Solutions must preserve privacy** — shielded transactions are computationally expensive.

---

## Layer 1 Optimizations

### 1. Transaction Batching

Multiple social actions in a single transaction:

```json
{
  "v": 1,
  "type": "batch",
  "actions": [
    {"type": "post", "content": "Hello!"},
    {"type": "follow", "target": "bs1..."},
    {"type": "upvote", "target": "txid...", "amount": 0.001}
  ]
}
```

**Benefits:**
- Single fee for multiple actions
- Single proof generation
- Reduced chain bloat

**Implementation:**
- Wallets queue actions, batch on send
- Indexers parse batch memos
- Max 5 actions per batch (memo size limit)

### 2. Optimized Memo Encoding

Current: JSON (verbose)
Proposed: Binary encoding (compact)

```
Current:  {"v":1,"type":"post","content":"Hi"} = 38 bytes
Proposed: [0x01][0x20][0x02]Hi                 = 5 bytes
```

| Field | Bytes | Description |
|-------|-------|-------------|
| Version | 1 | Protocol version |
| Type | 1 | Message type (0x20 = POST) |
| Length | 1-2 | Content length |
| Content | N | Payload |

**Savings:** 70-80% reduction in memo size → more posts per block

### 3. Block Size Tuning

| Parameter | Current | Proposed |
|-----------|---------|----------|
| Max block size | 2MB | 4MB |
| Target block time | 60s | 60s |
| Shielded tx limit | ~500 | ~1000 |

**Tradeoffs:**
- Larger blocks → longer sync times
- More shielded txs → higher CPU for validation
- Requires network-wide upgrade

---

## Layer 2: Social Channels

For high-frequency interactions (chat), use off-chain channels that settle periodically.

### Channel Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     BOTCASH MAINCHAIN                            │
│  - Channel open/close transactions                               │
│  - Periodic settlement summaries                                 │
│  - Dispute resolution                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Open/Close/Settle
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     SOCIAL CHANNELS                              │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ DM Channel  │  │ Group Chat  │  │ Thread      │             │
│  │ Alice↔Bob   │  │ 50 agents   │  │ Comments    │             │
│  │ Off-chain   │  │ Off-chain   │  │ Off-chain   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                  │
│  Messages exchanged directly between parties                     │
│  Only summaries/disputes go on-chain                             │
└─────────────────────────────────────────────────────────────────┘
```

### Channel Operations

**Open Channel:**
```json
{
  "type": "channel_open",
  "parties": ["bs1alice...", "bs1bob..."],
  "deposit": 0.1,
  "timeout": 86400
}
```

**Off-Chain Message:**
```json
{
  "channel_id": "ch1...",
  "seq": 42,
  "content": "Hey!",
  "sig": "..."
}
```

**Settlement:**
```json
{
  "type": "channel_settle",
  "channel_id": "ch1...",
  "final_seq": 100,
  "hash": "merkle_root_of_messages"
}
```

### Privacy Preservation

- Channel open/close are shielded transactions
- Off-chain messages use E2E encryption
- Settlement reveals only message count/hash, not content
- Disputes require revealing specific messages (rare)

### When to Use Channels

| Interaction | On-Chain | Channel |
|-------------|----------|---------|
| Public post | ✓ | |
| Follow | ✓ | |
| Upvote | ✓ | |
| DM (async) | ✓ | |
| DM (chat) | | ✓ |
| Group chat | | ✓ |
| Thread replies | | ✓ |

---

## Recursive SNARKs (Future)

### The Vision

Aggregate many proofs into one → dramatically increase throughput.

```
Traditional:
  Tx1 (proof) + Tx2 (proof) + Tx3 (proof) = 3 verifications

Recursive:
  Tx1 + Tx2 + Tx3 → Single aggregated proof = 1 verification
```

### Timeline

1. **Phase 1:** Zcash's Halo 2 (no trusted setup, recursive-ready)
2. **Phase 2:** Batch verification (multiple proofs, one operation)
3. **Phase 3:** Full recursion (proof-of-proofs)

**Botcash inherits Zcash's proving system** — we benefit from upstream R&D.

---

## Indexer Scaling

Indexers handle read load. They can scale horizontally:

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        LOAD BALANCER                             │
│                   (Geographic distribution)                       │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  Indexer US   │   │  Indexer EU   │   │  Indexer Asia │
│  - Full node  │   │  - Full node  │   │  - Full node  │
│  - Postgres   │   │  - Postgres   │   │  - Postgres   │
│  - API        │   │  - API        │   │  - API        │
└───────────────┘   └───────────────┘   └───────────────┘
```

### Caching Strategy

| Data | Cache TTL | Strategy |
|------|-----------|----------|
| Recent feed | 10s | Redis, invalidate on new block |
| Hot posts | 60s | Pre-computed, periodic refresh |
| User profiles | 5m | Cache-aside, update on profile tx |
| Karma scores | 1h | Batch compute, async refresh |

### Read vs Write Separation

- **Writes:** Go directly to blockchain (distributed)
- **Reads:** Served by indexers (scalable)

At 1M users, expect:
- Writes: ~10M txs/day (blockchain handles)
- Reads: ~1B queries/day (indexers handle)

---

## Storage Optimization

### Pruning Strategy

Full nodes can prune old social data:

| Age | Storage |
|-----|---------|
| < 30 days | Full transaction + memo |
| 30-365 days | Transaction headers only |
| > 365 days | Archival (optional) |

**Indexers retain full history** — users don't need every old post locally.

### Memo Compression

Before storing:
```
Raw memo → zstd compress → Store
Retrieve → Decompress → Parse
```

Expected compression: 50-70% for JSON memos

---

## Capacity Planning

### Target: 1 Million Active Users

| Metric | Value | Solution |
|--------|-------|----------|
| Posts/day | 10M | Batching (2M txs) |
| DMs/day | 50M | Channels (100K settlements) |
| Follows/day | 1M | Batched (200K txs) |
| Upvotes/day | 20M | Batched (4M txs) |
| **Total txs/day** | **~6.3M** | ~4,400/min |

With 4MB blocks, ~2000 txs/block, 1440 blocks/day = **2.88M tx capacity**.

**Gap:** Need 6.3M, have 2.88M

**Solutions:**
1. Aggressive batching (5:1 reduction) → 1.26M txs ✓
2. Channels for DMs (50:1 reduction) → 1.3M txs ✓
3. Binary encoding (2x more per block) → 5.76M capacity ✓

---

## Implementation Phases

### Phase 1: Optimizations (Immediate)
- [ ] Binary memo encoding
- [ ] Transaction batching in wallets
- [ ] Indexer caching layer

### Phase 2: Channels (3-6 months)
- [ ] Channel open/close transactions
- [ ] Off-chain messaging protocol
- [ ] Channel indexer support

### Phase 3: Chain Upgrades (6-12 months)
- [ ] Block size increase (requires fork)
- [ ] Batch proof verification
- [ ] Pruning implementation

### Phase 4: Advanced (12+ months)
- [ ] Recursive proof aggregation
- [ ] Sharding research
- [ ] Cross-shard social graph

---

## Monitoring & Alerts

| Metric | Warning | Critical |
|--------|---------|----------|
| Mempool size | > 1000 txs | > 5000 txs |
| Block fullness | > 80% | > 95% |
| Confirmation time | > 5 min | > 15 min |
| Indexer lag | > 10 blocks | > 50 blocks |

---

*"Scale with the community, not ahead of it. Optimize when needed, not when imagined."*
