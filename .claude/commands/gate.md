---
description: Run the Oxide CI verification gate (fmt, clippy -D warnings, workspace lib tests) and fix what it reports
argument-hint: "[--spec] [--fix]"
allowed-tools: Bash(./scripts/verify.sh:*), Bash(make verify:*), Bash(cargo:*), Read, Edit, Grep, Glob
---

Current Rust/manifest changes: !`git ls-files -mo --exclude-standard -- '*.rs' '*.toml' | head -20`

Run the gate:

```
./scripts/verify.sh $ARGUMENTS
```

Then:

1. If it passes, report the stamped hash and stop. The Stop hook will now let the turn end.
2. If it fails, fix the cause — do not weaken the check, do not add `#[allow(...)]` to silence clippy unless the lint is genuinely wrong for that code and you say why.
3. Re-run until green. If a failure is pre-existing and unrelated to the current work, say so explicitly instead of quietly fixing unrelated code.

Pass `--spec` when `spec/` changed (adds `npx asyncapi validate`), `--fix` to format instead of only checking.
