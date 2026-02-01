0a. Study @IMPLEMENTATION_PLAN.md to understand what needs to be built.
0b. Reference `specs/*` as needed (read specific files, don't bulk-scan).
0c. Botcash is a Zebra (Rust Zcash) fork with RandomX PoW. Uses Cargo workspaces.

1. Choose the most important unchecked task from @IMPLEMENTATION_PLAN.md. Search codebase before assuming something is missing. Use up to 10 parallel subagents for searches. Use 1 subagent for builds/tests.

2. Each task has "Required Tests:" — implement these. Tests are NOT optional. Task complete ONLY when required tests exist AND pass.

3. TARGETED TESTING (critical for performance):
   - Run ONLY the specific tests listed in "Required Tests:" for your task
   - `cargo check` - Fast syntax/type check (always safe)
   - `cargo test specific_test_name` - Run ONLY that test
   - `cargo test -p crate_name` - Run tests for ONE crate only
   - Do NOT run `cargo test` without filters (runs entire suite)

4. When tests pass, update @IMPLEMENTATION_PLAN.md (mark complete), `git add -A`, `git commit`.

CRITICAL: Required tests MUST exist and MUST pass before committing.
CRITICAL: Run TARGETED tests only — never the full test suite per task.
Important: No placeholders, stubs, or TODOs. Implement completely.
Important: Keep @IMPLEMENTATION_PLAN.md current with completion status.
Note: If you discover unrelated test failures, document them in IMPLEMENTATION_PLAN.md as new tasks — do NOT fix them in this increment.
