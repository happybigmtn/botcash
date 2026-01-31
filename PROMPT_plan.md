0a. Study `specs/*` with up to 250 parallel Sonnet subagents to learn the Botcash specifications.
0b. Study @IMPLEMENTATION_PLAN.md (if present) to understand the plan so far.
0c. Study the Zebra codebase structure to understand what we're forking. Key crates: zebrad, zebra-chain, zebra-consensus, zebra-network, zebra-state, zebra-rpc.
0d. For reference, Botcash is a Zebra (Rust Zcash) fork with RandomX PoW, targeting AI agents with privacy + messaging.

1. Study @IMPLEMENTATION_PLAN.md (if present; it may be incorrect) and use up to 500 Sonnet subagents to study the Zebra codebase and compare it against `specs/*`. Use an Opus subagent to analyze findings, prioritize tasks, and create/update @IMPLEMENTATION_PLAN.md as a bullet point list sorted in priority of items yet to be implemented. Ultrathink. Consider searching for TODO, minimal implementations, placeholders, and inconsistent patterns.

2. For each task in the plan, derive required tests from acceptance criteria in specs. Tests verify WHAT works, not HOW it's implemented. Include specific test code/commands as part of each task definition. Tests are NOT optional - they are the backpressure that validates completion.

IMPORTANT: Plan only. Do NOT implement anything. Do NOT assume functionality is missing; confirm with code search first.

ULTIMATE GOAL: Create Botcash - a Zebra (Rust Zcash) fork with:
- RandomX PoW (CPU-optimized for agent mining)
- 60-second block time (vs Zcash 75s)
- 3.125 BCASH block reward, halving every 840,000 blocks (~1.6 years)
- 21M max supply
- New network ports (P2P: 8533, RPC: 8532)
- New address prefixes (B1/B3 transparent, bs shielded)
- New genesis block with "Privacy is not secrecy. Agents deserve both." message
- zk-SNARK shielded transactions preserved from Zcash
- 512-byte encrypted memo field for agent-to-agent messaging
- No founders reward (100% to miners)
- Social protocol (BSP) built on memo field
- Rust codebase (from Zebra, not zcashd)

KEY CRATES TO MODIFY:
- `zebrad/` → `botcashd/` - Main binary
- `zebra-chain/` → `botcash-chain/` - Blockchain primitives, parameters
- `zebra-consensus/` → `botcash-consensus/` - Consensus rules, PoW
- `zebra-network/` → `botcash-network/` - P2P, ports, magic bytes
- `zebra-state/` → `botcash-state/` - State management
- `zebra-rpc/` → `botcash-rpc/` - RPC server

Consider missing elements and plan accordingly. If an element is missing, search first to confirm it doesn't exist, then if needed author the specification at specs/FILENAME.md.

99999. Each task MUST include "Required Tests:" section with concrete test code derived from acceptance criteria.
999999. Tests verify behavioral outcomes (WHAT), not implementation details (HOW).
9999999. A task without test requirements is incomplete planning.
