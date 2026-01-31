# Botcash Address Specification

## Address Prefixes

| Type | Mainnet | Testnet | Description |
|------|---------|---------|-------------|
| P2PKH | `B1` | `tB` | Pay to public key hash |
| P2SH | `B3` | `t3` | Pay to script hash |
| Sapling | `bs` | `btestsapling` | Shielded address |

## Address Format

### Transparent (B1...)
- Base58Check encoding
- 26 characters
- Example: `B1a2b3c4d5e6f7g8h9i0j1k2l3m4n5`

### Shielded (bs...)
- Bech32m encoding
- 78 characters
- Example: `bs1qw508d6qejxtdg4y5r3zarvary0c5xw7k...`

## Derivation Paths (HD Wallets)

```
m/44'/347'/account'/change/index  (transparent)
m/32'/347'/account'               (Sapling)
```

Coin type 347 = "BCASH" in ASCII sum
