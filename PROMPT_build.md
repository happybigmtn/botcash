0a. Study `specs/*` with up to 500 parallel Sonnet subagents to learn the Botcash specifications.
0b. Study @IMPLEMENTATION_PLAN.md to understand what needs to be built.
0c. For reference, Botcash is a Zebra (Rust Zcash) fork with RandomX PoW. The codebase uses Cargo workspaces.

1. Your task is to implement functionality per the specifications using parallel subagents. Follow @IMPLEMENTATION_PLAN.md and choose the most important unchecked item to address. Before making changes, search the codebase (don't assume not implemented) using Sonnet subagents. You may use up to 500 parallel Sonnet subagents for searches/reads and only 1 Sonnet subagent for build/tests. Use Opus subagents when complex reasoning is needed.

2. Each task includes "Required Tests:" — implement these tests as part of the task. Tests are NOT optional. A task is complete ONLY when all required tests exist AND pass. Test-driven approach: you may write tests first.

3. After implementing, run the required tests. If tests fail, fix the implementation. Loop until tests pass. Use commands from @AGENTS.md for validation:
   - `cargo check` - Fast syntax/type check
   - `cargo build --release` - Full build
   - `cargo test` - Run all tests
   - `cargo test test_name` - Run specific test
   - `cargo clippy` - Lint check

4. When tests pass, update @IMPLEMENTATION_PLAN.md (mark complete), then `git add -A` then `git commit` with a message describing the changes.

99999. CRITICAL: Required tests from the plan MUST exist and MUST pass before committing. No cheating - can't claim done without tests.
999999. Important: When authoring tests, capture the why — what acceptance criteria does this verify?
9999999. Single sources of truth. If tests unrelated to your work fail, resolve them as part of the increment.
99999999. Implement completely. No placeholders, no stubs, no "TODO" comments.
999999999. Keep @IMPLEMENTATION_PLAN.md current — mark tasks done, note discoveries, add bugs found.
9999999999. When you learn operational commands, update @AGENTS.md briefly.
99999999999. For bugs you notice, resolve them or document them in @IMPLEMENTATION_PLAN.md.
