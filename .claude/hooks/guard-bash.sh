#!/usr/bin/env bash
# PreToolUse (Bash): block the handful of commands that can silently destroy
# work or bypass the project's quality gates.
set -uo pipefail

input=$(cat)
cmd=$(jq -r '.tool_input.command // empty' <<<"$input")
[ -n "$cmd" ] || exit 0

deny() {
  jq -n --arg r "$1" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "deny",
      permissionDecisionReason: $r
    }
  }'
  exit 0
}

ask() {
  jq -n --arg r "$1" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "ask",
      permissionDecisionReason: $r
    }
  }'
  exit 0
}

# Bypassing pre-commit skips the same gate CLAUDE.md requires.
if [[ "$cmd" =~ git[[:space:]]+commit ]] && [[ "$cmd" =~ (--no-verify|[[:space:]]-n([[:space:]]|$)) ]]; then
  deny "git commit --no-verify bypasses the pre-commit gate (fmt/clippy/check/test). Run \`make verify\`, fix what it reports, then commit normally."
fi

# Force pushes rewrite published history on a solo-dev main branch.
if [[ "$cmd" =~ git[[:space:]]+push ]] && [[ "$cmd" =~ (--force([[:space:]]|$)|[[:space:]]-f([[:space:]]|$)) ]]; then
  deny "Force push blocked. Use --force-with-lease and only after confirming with the user."
fi

# Proprietary license, not published to crates.io.
if [[ "$cmd" =~ cargo[[:space:]]+publish ]]; then
  deny "Oxide CI is proprietary and installed from source; cargo publish is not part of this project's workflow."
fi

# Recursive deletes anywhere near the repo or home.
if [[ "$cmd" =~ rm[[:space:]]+(-[a-zA-Z]*[rR][a-zA-Z]*[[:space:]]+)+ ]]; then
  if [[ "$cmd" =~ rm[[:space:]]+-[a-zA-Z]*[rR][a-zA-Z]*[[:space:]]+(/|~|\$HOME|\.\.)([[:space:]]|$|/) ]]; then
    deny "Recursive delete of an absolute/home path is blocked."
  fi
  if [[ "$cmd" =~ (crates|spec|docs|examples|scripts|\.oxide-ci|\.claude|\.github) ]]; then
    ask "This recursively deletes tracked project directories — confirm before running."
  fi
fi

# Throwing away uncommitted work.
if [[ "$cmd" =~ git[[:space:]]+reset[[:space:]]+--hard ]] || [[ "$cmd" =~ git[[:space:]]+clean[[:space:]]+-[a-zA-Z]*f ]]; then
  ask "This discards uncommitted changes irreversibly — confirm before running."
fi

exit 0
