---
description: Work a GitHub issue end to end following the Oxide CI solo-dev workflow
argument-hint: "[issue-number]"
disable-model-invocation: true
---

Work issue #$ARGUMENTS to completion.

Branch: !`git rev-parse --abbrev-ref HEAD` — dirty: !`git status --porcelain | wc -l` files

## Steps

1. **Read the ticket.** `gh issue view $ARGUMENTS`. Restate the acceptance criteria as a checklist. If criteria are missing or unverifiable, ask before writing code.
2. **Branch.** From an up-to-date `main`: `git checkout main && git pull && git checkout -b feat/issue-$ARGUMENTS-<slug>` (use `fix/` for defects). Skip if already on that branch. Refuse to start if the working tree has unrelated uncommitted changes.
3. **Implement.** Respect the architecture rules in CLAUDE.md — domain in `oxide-core`, integrations in their own adapter crate, spec change alongside any event change.
4. **Test.** Add tests with the change, not after. Delegate to `test-author` for anything substantial.
5. **Verify.** `./scripts/verify.sh` (add `--spec` if `spec/` changed). Green is required.
6. **Review.** Delegate the diff to `rust-reviewer`; if `spec/` or event types changed, also `spec-correlator`. Address real findings.
7. **Evidence.** Walk each acceptance criterion and name the proof — test, command output, or file. Report any you cannot evidence and stop there.
8. **Commit.** Atomic conventional commits, final one `Closes #$ARGUMENTS`. Never `--no-verify`.
9. **Land.** Ask before merging to `main` and pushing. After merge: delete the branch, then close the issue with a summary comment.

Do not skip step 7. A ticket is not done because the code compiles.
