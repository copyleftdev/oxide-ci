#!/usr/bin/env bash
# SessionStart: put the working state in front of Claude before the first turn —
# branch, dirty files, gate status, and the issue the branch name points at.
set -uo pipefail

cat >/dev/null   # drain stdin

ROOT="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}"
[ -d "$ROOT" ] || exit 0
cd "$ROOT" || exit 0

branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "?")
dirty=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
rust_dirty=$(git ls-files -mo --exclude-standard -- '*.rs' '*.toml' 2>/dev/null | wc -l | tr -d ' ')

gate="n/a (no Rust changes)"
if [ "$rust_dirty" != "0" ]; then
  current=$(./scripts/verify.sh --hash 2>/dev/null || echo "x")
  stamped=$(cat target/.oxide-verify 2>/dev/null || echo "")
  if [ "$current" = "$stamped" ]; then gate="PASSED for current tree"; else gate="NOT PASSED — run \`make verify\` before finishing"; fi
fi

build="cold (target/ absent — first build will take minutes)"
[ -d target ] && build="warm"

issue=""
if [[ "$branch" =~ issue-([0-9]+) ]]; then
  n="${BASH_REMATCH[1]}"
  title=$(timeout 5 gh issue view "$n" --json title -q .title 2>/dev/null || echo "")
  [ -n "$title" ] && issue="Branch tracks issue #$n: $title"
fi

ctx="Repo state — branch: $branch | dirty files: $dirty (Rust/manifest: $rust_dirty) | gate: $gate | build cache: $build"
[ -n "$issue" ] && ctx="$ctx
$issue"

jq -n --arg c "$ctx" '{
  hookSpecificOutput: {
    hookEventName: "SessionStart",
    additionalContext: $c
  }
}'
