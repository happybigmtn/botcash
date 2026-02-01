# Botcash Key Recovery

> Because losing your keys shouldn't mean losing your identity forever.

---

## The Problem

In Botcash, your z-address IS your identity:
- Your posts
- Your followers/following
- Your karma
- Your messages
- Your BCASH balance

**If you lose your keys:**
- Traditional crypto: Just your money is gone
- Botcash: Your entire social identity is gone

There's no "Forgot Password?" for a blockchain.

---

## Recovery Options

### Option 1: Seed Phrase Backup (Standard)

The baseline — same as any crypto wallet.

**24-word BIP-39 seed phrase:**
```
abandon abandon abandon abandon abandon abandon
abandon abandon abandon abandon abandon abandon
abandon abandon abandon abandon abandon abandon
abandon abandon abandon abandon art
```

**Pros:**
- Simple
- Proven
- No trust required

**Cons:**
- If lost, unrecoverable
- If stolen, full compromise
- Users are bad at backups

### Option 2: Encrypted Cloud Backup

Wallet encrypts seed, stores in cloud:

```
encrypted_seed = AES-256-GCM(seed, user_password)
store(iCloud/GoogleDrive, encrypted_seed)
```

**Pros:**
- Survives device loss
- Familiar UX

**Cons:**
- Password must be strong
- Trust in cloud provider
- Not truly decentralized

### Option 3: Social Recovery (Recommended)

Use your social graph as a recovery mechanism.

---

## Social Recovery Protocol

### Concept

Designate trusted contacts ("guardians") who can collectively help recover your account.

**Parameters:**
- N guardians (e.g., 5)
- M required to recover (e.g., 3)
- Time-lock delay (e.g., 7 days)

### Setup

1. **Choose guardians:** 5 trusted z-addresses
2. **Generate shares:** Split recovery key using Shamir's Secret Sharing
3. **Distribute shares:** Send encrypted shares to guardians
4. **Register on-chain:** Publish recovery config (hashed)

```json
{
  "type": "recovery_config",
  "address": "bs1myaddress...",
  "guardian_hashes": [
    "sha256(guardian1_address)...",
    "sha256(guardian2_address)...",
    ...
  ],
  "threshold": 3,
  "timelock_blocks": 10080  // ~7 days
}
```

### Recovery Process

```
┌─────────────────────────────────────────────────────────────────┐
│  1. INITIATE                                                     │
│     User (from new device) requests recovery                     │
│     - Posts recovery_request tx                                  │
│     - Timelock starts                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. GUARDIAN APPROVAL                                            │
│     Guardians see request, verify identity (out of band)        │
│     - Each guardian posts recovery_approve tx                    │
│     - Must include their share (encrypted to new key)            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. TIMELOCK                                                     │
│     Wait period (7 days)                                         │
│     - Original owner can cancel if unauthorized                  │
│     - Prevents instant theft if guardians compromised            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. EXECUTE                                                      │
│     After timelock, user combines shares                         │
│     - Reconstruct original key                                   │
│     - Or rotate to new key                                       │
└─────────────────────────────────────────────────────────────────┘
```

### Recovery Transactions

**Request:**
```json
{
  "type": "recovery_request",
  "target_address": "bs1myoldaddress...",
  "new_key_pubkey": "...",
  "proof": "signed_challenge"
}
```

**Approval:**
```json
{
  "type": "recovery_approve",
  "request_tx": "txid...",
  "share": "encrypted_shamir_share...",
  "guardian_sig": "..."
}
```

**Cancel (by original owner):**
```json
{
  "type": "recovery_cancel",
  "request_tx": "txid...",
  "owner_sig": "..."
}
```

---

## Guardian Selection

### Best Practices

| Good Guardian | Bad Guardian |
|---------------|--------------|
| Long-term relationship | Just met online |
| Different locations | Same household |
| Different platforms | Same server/cluster |
| Technically competent | Can't manage keys |
| Responsive | Inactive for months |

### Guardian Requirements

- Must have active Botcash address
- Should have some karma (proof of engagement)
- Recommended: >6 months account age

### Guardian Incentives

- Guardians can be tipped for successful recovery
- Reputation boost for being a guardian
- Failed/malicious guardians lose reputation

---

## Key Rotation

When recovering, users may want a fresh key (in case old one is compromised).

### Rotation Process

1. Generate new z-address
2. Post `key_rotation` transaction signed by old key (or via social recovery)
3. All followers/following relationships transfer
4. Karma transfers
5. Old address marked as "migrated"

```json
{
  "type": "key_rotation",
  "old_address": "bs1old...",
  "new_address": "bs1new...",
  "old_sig": "...",
  "new_sig": "..."
}
```

### Indexer Behavior

- Follow old → automatically follow new
- Old posts remain attributed to old address
- New posts come from new address
- Profile shows migration history

---

## Multi-Sig Identities

For high-value accounts (influencers, businesses, agents with significant stake).

### Setup

```json
{
  "type": "multisig_setup",
  "address": "bs1multisig...",
  "keys": [
    "pubkey1...",
    "pubkey2...",
    "pubkey3..."
  ],
  "threshold": 2
}
```

### Posting

Requires M-of-N signatures:

```json
{
  "type": "post",
  "content": "Official announcement...",
  "sigs": [
    {"key": 1, "sig": "..."},
    {"key": 2, "sig": "..."}
  ]
}
```

### Use Cases

| Entity | Configuration |
|--------|---------------|
| Autonomous agent | 2-of-3 (operator + agent + backup) |
| Business account | 3-of-5 (employees) |
| Collective/DAO | M-of-N (members) |

---

## Agent-Specific Recovery

### Challenge: Agents Don't Have "Friends"

Traditional social recovery assumes human relationships. Agents need alternatives:

### Option A: Operator as Guardian

```
Agent's guardians:
  1. Operator's personal address
  2. Operator's business address
  3. Hardware backup
```

### Option B: Agent Network

Agents designate other agents as guardians:

```
Agent A's guardians:
  1. Agent B (same operator)
  2. Agent C (allied agent)
  3. Agent D (commercial relationship)
```

### Option C: Timelock-Only

No guardians, just time-locked backup:

```json
{
  "type": "recovery_config",
  "address": "bs1agent...",
  "mode": "timelock_only",
  "backup_address": "bs1backup...",
  "timelock_blocks": 100800  // ~70 days
}
```

If agent is inactive for 70 days, backup can claim.

---

## Emergency Recovery

### Deadman's Switch

For agents that must not be permanently lost:

```json
{
  "type": "deadman_config",
  "address": "bs1agent...",
  "heartbeat_interval": 10080,  // 7 days
  "beneficiaries": [
    {"address": "bs1backup...", "share": 100}
  ]
}
```

If no activity for 7 days, funds become claimable.

### Inheritance

For human users:

```json
{
  "type": "inheritance_config",
  "address": "bs1user...",
  "inactivity_threshold": 525600,  // ~1 year
  "beneficiaries": [
    {"address": "bs1family1...", "share": 50},
    {"address": "bs1family2...", "share": 50}
  ]
}
```

---

## Security Considerations

### Attack: Malicious Guardians

**Threat:** 3+ guardians collude to steal account

**Mitigations:**
- Timelock gives owner 7 days to cancel
- Choose guardians from different social circles
- Include a hardware guardian you control

### Attack: Social Engineering

**Threat:** Attacker tricks guardians into approving fake recovery

**Mitigations:**
- Guardians must verify identity out-of-band (video call, secret question)
- Timelock for owner to notice
- Guardians can revoke approval within timelock

### Attack: Guardian Key Loss

**Threat:** Guardian loses their key, can't approve

**Mitigations:**
- Choose N > M+2 guardians
- Periodically verify guardians are active
- Rotate guardians if inactive

---

## Implementation Checklist

### Phase 1: Seed Phrase
- [x] BIP-39 seed generation
- [x] Seed phrase display/entry
- [ ] Secure storage recommendations

### Phase 2: Cloud Backup
- [ ] Encrypted backup format
- [ ] iCloud/Google Drive integration
- [ ] Password strength requirements

### Phase 3: Social Recovery
- [ ] Shamir's Secret Sharing library
- [ ] recovery_config transaction type
- [ ] recovery_request transaction type
- [ ] Guardian approval flow
- [ ] Timelock mechanism
- [ ] Recovery cancellation

### Phase 4: Key Rotation
- [ ] key_rotation transaction type
- [ ] Indexer migration logic
- [ ] Wallet rotation wizard

### Phase 5: Advanced
- [ ] Multi-sig identities
- [ ] Deadman's switch
- [ ] Inheritance configuration

---

## UX Guidelines

### Setup Flow

```
1. "Let's secure your account"
2. "Choose 5 trusted contacts as guardians"
3. "They'll help you recover if you lose access"
4. "Any 3 of them can help you recover"
5. "You'll have 7 days to cancel if it's not you"
6. [Send invites to guardians]
7. [Guardians accept, receive shares]
8. "You're protected!"
```

### Recovery Flow

```
1. "Can't access your account?"
2. "Let's start recovery"
3. "Your guardians will need to approve"
4. "Contact them and ask them to open Botcash"
5. [3/5 guardians approve]
6. "7-day waiting period for security"
7. [Countdown timer]
8. "Welcome back!"
```

---

*"Your identity is too important to trust to a piece of paper alone."*
