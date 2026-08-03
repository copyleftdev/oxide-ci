#!/usr/bin/env bash
# Oxide CI verification gate.
#
# Runs the same checks as .pre-commit-config.yaml and the `quality` stage of
# .oxide-ci/pipeline.yaml, then stamps a ledger file so the Claude Code Stop
# hook (.claude/hooks/verify-gate.sh) knows this exact working tree passed.
#
#   ./scripts/verify.sh          fmt --check, clippy -D warnings, workspace lib tests
#   ./scripts/verify.sh --fix    format first, then the above
#   ./scripts/verify.sh --spec   also validate the AsyncAPI spec (needs network/npx)
#   ./scripts/verify.sh --hash   print the tree hash and exit (used by hooks)
#
# Exit 0 = gate passed and ledger stamped. Non-zero = gate failed, no stamp.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || dirname "$(dirname "$(readlink -f "$0")")")"
cd "$ROOT" || exit 1

STAMP="$ROOT/target/.oxide-verify"

# Hash of everything that can affect the gate result: HEAD plus the content of
# every modified or untracked Rust/manifest file.
tree_hash() {
  {
    git rev-parse HEAD 2>/dev/null || echo "no-head"
    git ls-files -mo --exclude-standard -- '*.rs' '*.toml' 2>/dev/null | sort | while read -r f; do
      [ -f "$f" ] && sha256sum "$f"
    done
    git diff --cached --name-only -- '*.rs' '*.toml' 2>/dev/null | sort | while read -r f; do
      [ -f "$f" ] && sha256sum "$f"
    done
  } | sha256sum | cut -d' ' -f1
}

case "${1:-}" in
  --hash) tree_hash; exit 0 ;;
esac

FIX=0
SPEC=0
for arg in "$@"; do
  case "$arg" in
    --fix) FIX=1 ;;
    --spec) SPEC=1 ;;
    *) echo "unknown flag: $arg" >&2; exit 64 ;;
  esac
done

step() {
  local label="$1"; shift
  echo "▸ $label"
  if ! "$@"; then
    echo "✗ gate failed at: $label" >&2
    exit 1
  fi
}

if [ "$FIX" = 1 ]; then
  step "cargo fmt --all" cargo fmt --all
else
  step "cargo fmt --check" cargo fmt --all -- --check
fi

step "cargo clippy -D warnings" cargo clippy --workspace --all-targets -- -D warnings
step "cargo test --workspace --lib --bins" cargo test --workspace --lib --bins

if [ "$SPEC" = 1 ]; then
  step "asyncapi validate" npx --yes asyncapi validate spec/asyncapi.yaml
fi

mkdir -p "$ROOT/target"
tree_hash > "$STAMP"
echo "✓ gate passed — ledger stamped ($(cat "$STAMP" | cut -c1-12))"
