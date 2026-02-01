#!/usr/bin/env bash
# Ralph loop runner - iteratively implement tasks from IMPLEMENTATION_PLAN.md
set -euo pipefail

# Resolve script directory and repo root
# Script lives in repo root for botcash (not a subdirectory like rsocietyv2/ralph/)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$SCRIPT_DIR"

# Configuration - paths relative to SCRIPT_DIR for prompts, REPO_ROOT for execution
PLAN_FILE="$SCRIPT_DIR/IMPLEMENTATION_PLAN.md"
LOG_DIR="$SCRIPT_DIR/logs"

# Determine prompt file based on mode
MODE="build"
WORK_SCOPE=""
PROMPT_FILE="$SCRIPT_DIR/PROMPT_build.md"
MAX_ITERATIONS=0  # 0 = unlimited

if [[ "${1:-}" == "plan" ]]; then
    MODE="plan"
    PROMPT_FILE="$SCRIPT_DIR/PROMPT_plan.md"
    MAX_ITERATIONS=${2:-0}
elif [[ "${1:-}" == "plan-work" ]]; then
    MODE="plan-work"
    PROMPT_FILE="$SCRIPT_DIR/PROMPT_plan_work.md"
    WORK_SCOPE="${2:-}"
    MAX_ITERATIONS=${3:-0}
    if [[ -z "$WORK_SCOPE" ]]; then
        echo "Error: plan-work requires a scope description"
        echo "Usage: ./loopclaude.sh plan-work \"description of work scope\""
        exit 1
    fi
elif [[ "${1:-}" =~ ^[0-9]+$ ]]; then
    MAX_ITERATIONS=$1
fi

mkdir -p "$LOG_DIR"

count_remaining() {
    # Support both checkbox format (- [ ]) and table format (⬜ TODO)
    local checkbox_count table_count
    checkbox_count=$(grep -c '^- \[ \]' "$PLAN_FILE" 2>/dev/null) || checkbox_count=0
    table_count=$(grep -c '⬜ TODO' "$PLAN_FILE" 2>/dev/null) || table_count=0
    echo $((checkbox_count + table_count))
}

count_completed() {
    # Support both checkbox format (- [x]) and table format (✅ DONE)
    local checkbox_count table_count
    checkbox_count=$(grep -c '^- \[x\]' "$PLAN_FILE" 2>/dev/null) || checkbox_count=0
    table_count=$(grep -c '✅ DONE' "$PLAN_FILE" 2>/dev/null) || table_count=0
    echo $((checkbox_count + table_count))
}

count_blocked() {
    local count
    count=$(grep -c 'Blocked:' "$PLAN_FILE" 2>/dev/null) || count=0
    echo "$count"
}

# Verify files exist
if [[ ! -f "$PROMPT_FILE" ]]; then
    echo "Error: $PROMPT_FILE not found"
    exit 1
fi

if [[ ! -f "$PLAN_FILE" ]]; then
    echo "Error: $PLAN_FILE not found"
    exit 1
fi

# Header
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Ralph Loop"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Mode:   $MODE"
echo "  Root:   $REPO_ROOT"
echo "  Prompt: $PROMPT_FILE"
echo "  Plan:   $PLAN_FILE"
[[ "$MAX_ITERATIONS" -gt 0 ]] && echo "  Max:    $MAX_ITERATIONS iterations"
[[ "$MAX_ITERATIONS" -eq 0 ]] && echo "  Max:    unlimited"
[[ -n "$WORK_SCOPE" ]] && echo "  Scope:  $WORK_SCOPE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

iteration=0
while true; do
    iteration=$((iteration + 1))
    remaining=$(count_remaining)
    completed=$(count_completed)
    timestamp=$(date +%Y%m%d-%H%M%S)
    log_file="$LOG_DIR/run-${iteration}-${timestamp}.log"

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Iteration $iteration | Remaining: $remaining | Completed: $completed"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if [[ "$remaining" -eq 0 ]]; then
        echo "All tasks complete!"
        break
    fi

    if [[ "$MAX_ITERATIONS" -gt 0 ]] && [[ "$iteration" -gt "$MAX_ITERATIONS" ]]; then
        echo "Max iterations ($MAX_ITERATIONS) reached. $remaining tasks remaining."
        exit 1
    fi

    # Build prompt with scope substitution for plan-work mode
    if [[ "$MODE" == "plan-work" ]]; then
        export WORK_SCOPE
        prompt_content=$(envsubst < "$PROMPT_FILE")
    else
        prompt_content=$(cat "$PROMPT_FILE")
    fi

    # Capture plan hash before run (for plan mode progress detection)
    plan_hash_before=$(md5sum "$PLAN_FILE" | cut -d' ' -f1)

    # Run Claude Code from repo root
    echo "Running Claude Code from $REPO_ROOT..."
    pushd "$REPO_ROOT" > /dev/null
    # Use --output-format json to avoid streaming mode issues with -p flag
    # Redirect stderr to capture errors, tee stdout to log file
    if claude --dangerously-skip-permissions --output-format json -p "$prompt_content" > "$log_file" 2>&1; then
        echo "Claude Code completed successfully"
        # Show summary from JSON output
        if command -v jq &>/dev/null && [[ -s "$log_file" ]]; then
            jq -r 'select(.type == "result") | "Cost: $\(.total_cost_usd // 0 | tostring | .[0:6]) | Turns: \(.num_turns // 0)"' "$log_file" 2>/dev/null || true
        fi
    else
        if grep -q "Error: No messages returned" "$log_file" 2>/dev/null; then
            echo "Claude Code returned no messages (likely transient). Retrying after 5s..."
            sleep 5
            continue
        fi
        echo "Claude Code exited with error, checking if progress was made..."
        # Show error from JSON output
        if command -v jq &>/dev/null && [[ -s "$log_file" ]]; then
            jq -r '.errors[]? // empty' "$log_file" 2>/dev/null | head -5 || true
        fi
    fi
    popd > /dev/null

    # Check progress - different logic for plan vs build mode
    if [[ "$MODE" == "plan" ]] || [[ "$MODE" == "plan-work" ]]; then
        # Plan mode: progress = file was modified
        plan_hash_after=$(md5sum "$PLAN_FILE" | cut -d' ' -f1)
        if [[ "$plan_hash_after" == "$plan_hash_before" ]]; then
            echo "No progress made this iteration (plan unchanged). Check $log_file"
            echo "Waiting 10s before retry..."
            sleep 10
        else
            new_remaining=$(count_remaining)
            echo "Plan updated: $remaining -> $new_remaining tasks"
        fi
    else
        # Build mode: progress = tasks completed
        new_remaining=$(count_remaining)
        if [[ "$new_remaining" -eq "$remaining" ]]; then
            echo "No progress made this iteration. Check $log_file"
            echo "Waiting 10s before retry..."
            sleep 10
        else
            echo "Progress: $remaining -> $new_remaining tasks"
        fi
    fi

    # Auto-commit if enabled (from repo root)
    if [[ "${RALPH_AUTOCOMMIT:-0}" == "1" ]] && [[ "$MODE" == "build" ]]; then
        if [[ -n "$(git -C "$REPO_ROOT" status --porcelain)" ]]; then
            msg="ralph: iteration $iteration"
            echo "Committing: $msg"
            git -C "$REPO_ROOT" add -A && git -C "$REPO_ROOT" commit -m "$msg" || echo "Commit failed"
        fi
    fi

    sleep 2
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Total iterations: $iteration"
echo "  Tasks completed:  $(count_completed)"
echo "  Tasks blocked:    $(count_blocked)"
echo "  Logs:             $LOG_DIR/"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
