#!/usr/bin/env bash
# PostToolUse (Edit|Write|MultiEdit): format Rust on the spot, and remind about
# spec/code correlation when the AsyncAPI spec is touched.
set -uo pipefail

input=$(cat)
file=$(jq -r '.tool_input.file_path // .tool_input.notebook_path // empty' <<<"$input")
[ -n "$file" ] || exit 0
[ -f "$file" ] || exit 0

ROOT="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}"
rel="${file#"$ROOT"/}"

case "$file" in
  *.rs)
    if ! err=$(rustfmt --edition 2024 --quiet "$file" 2>&1); then
      jq -n --arg e "$err" '{
        hookSpecificOutput: {
          hookEventName: "PostToolUse",
          additionalContext: ("rustfmt could not format this file (usually a syntax error): " + $e)
        }
      }'
    fi
    ;;
esac

case "$rel" in
  spec/*.yaml|spec/*.yml|spec/*/*.yaml|spec/*/*.yml)
    jq -n '{
      hookSpecificOutput: {
        hookEventName: "PostToolUse",
        additionalContext: "AsyncAPI spec changed. Before finishing: (1) run `make lint` to validate the spec, (2) update the matching Rust type in oxide-core, (3) keep the `spec_link!` correlation and its test in sync, (4) update the relevant `_index.yaml`."
      }
    }'
    ;;
esac
exit 0
