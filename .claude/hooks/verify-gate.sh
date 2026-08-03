#!/usr/bin/env bash
# Stop hook: refuse to end the turn while Rust changes are unverified.
#
# Cheap by design — it never compiles anything. It compares the current tree
# hash against the ledger written by scripts/verify.sh and, if they differ,
# blocks the stop and tells Claude to run the gate.
#
# Escape hatches: OXIDE_GATE=off, or two consecutive blocks in one session.
set -uo pipefail

input=$(cat)
[ "${OXIDE_GATE:-on}" = "off" ] && exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}"
[ -d "$ROOT" ] || exit 0
cd "$ROOT" || exit 0

# Nothing Rust-shaped is dirty -> nothing to gate.
dirty=$(git ls-files -mo --exclude-standard -- '*.rs' '*.toml' 2>/dev/null | head -1)
staged=$(git diff --cached --name-only -- '*.rs' '*.toml' 2>/dev/null | head -1)
[ -z "$dirty" ] && [ -z "$staged" ] && exit 0

current=$(./scripts/verify.sh --hash 2>/dev/null) || exit 0
stamped=$(cat target/.oxide-verify 2>/dev/null || echo "")
[ "$current" = "$stamped" ] && exit 0

# Loop breaker: don't block the same session more than twice.
session=$(jq -r '.session_id // "unknown"' <<<"$input")
counter="target/.oxide-gate-$session"
mkdir -p target
attempts=$(cat "$counter" 2>/dev/null || echo 0)
[[ "$attempts" =~ ^[0-9]+$ ]] || attempts=0
if [ "$attempts" -ge 2 ]; then
  jq -n '{systemMessage: "Gate still unverified after 2 reminders — allowing stop. Run `make verify` before committing."}'
  exit 0
fi
echo $((attempts + 1)) > "$counter"

jq -n '{
  decision: "block",
  reason: "Rust/manifest files changed but the verification gate has not passed for this tree. Run `make verify` (cargo fmt --check, clippy -D warnings, cargo test --workspace --lib) and fix what it reports. If the change is intentionally unverifiable right now, say so explicitly and set OXIDE_GATE=off for the run."
}'
exit 0
