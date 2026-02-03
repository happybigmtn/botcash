0a. Study @IMPLEMENTATION_PLAN.md to understand what needs to be built.
0b. Reference `specs/*` as needed (read specific files, don't bulk-scan).
0c. Botcash is a Zebra (Rust Zcash) fork with RandomX PoW. Uses Cargo workspaces.

## Task Selection

1. Find unchecked tasks (`- [ ]`) in @IMPLEMENTATION_PLAN.md
2. Choose ONE task with clear implementation scope
3. Search codebase before assuming something is missing
4. Use up to 10 parallel subagents for searches, 1 for builds/tests

## Required Tests

Each task has "Required Tests:" — implement these. Tests are NOT optional.
Task complete ONLY when required tests exist AND pass.

## Targeted Testing (CRITICAL)

Run ONLY tests for YOUR specific task — nothing else.

**NEVER run workspace-level commands:**
✗ `cargo test` (runs all Rust tests across entire workspace)
✗ `cargo test --all` (same problem)
✗ `cargo test --workspace` (same problem)

**ALWAYS use filters:**
✓ `cargo check` - Fast syntax/type check (always safe)
✓ `cargo test specific_test_name` - One test only
✓ `cargo test -p botcash-chain test_name` - One test in one crate
✓ `cargo test -p botcash-consensus -- --test-threads=1 test_verify` - Single test

IGNORE unrelated test failures — document them as new tasks.

## Marking Complete

1. BEFORE committing, edit @IMPLEMENTATION_PLAN.md
2. Change `- [ ] ...` to `- [x] ...`
3. Then `git add -A`, `git commit -m "feat: description"`

## Rules

- CRITICAL: Required tests MUST exist and MUST pass before committing
- CRITICAL: Run TARGETED tests only — never workspace-level commands
- CRITICAL: Mark task complete in IMPLEMENTATION_PLAN.md before committing
- Important: No placeholders, stubs, or TODOs - implement completely
- Note: Document unrelated test failures as new tasks in IMPLEMENTATION_PLAN.md
