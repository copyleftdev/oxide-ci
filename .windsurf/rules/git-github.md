---
trigger: model_decision
description: Apply when user is working with git, GitHub, commits, branches, PRs, or issues
---

# Git & GitHub Rules

Rules for version control and GitHub workflow. This is a solo developer workflow — no PRs needed, merge directly to main.

## Commit Messages

<commits>
- Use conventional commits format
- Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `ci`
- Format: `type(scope): description`
- **Always reference issue**: `Refs #123` or `Closes #123`
- Use `Closes #123` in final commit to auto-close issue
- Examples:
  - `feat(oxide-api): add pipeline CRUD endpoints - Closes #5`
  - `fix(oxide-runner): handle container timeout correctly - Fixes #42`
  - `docs: update ARCHITECTURE.md with cache flow - Refs #21`
  - `test(oxide-core): add serialization tests for events - Refs #2`
</commits>

## Branch Strategy (Solo Dev)

<branches>
- `main` - stable, always deployable
- `feat/issue-{number}-{description}` - feature branches
- `fix/issue-{number}-{description}` - bug fix branches
- Always branch from latest `main`
- Merge directly to `main` when complete (no PR needed)
- Delete branch immediately after merge
- Keep branch lifetime short — complete and merge within a session when possible
</branches>

## Branch Commands

<branch_commands>
```bash
# Start work on issue #5
git checkout main && git pull
git checkout -b feat/issue-5-api-endpoints

# Complete work and merge
git checkout main && git pull
git merge feat/issue-5-api-endpoints
git push origin main

# Clean up
git branch -d feat/issue-5-api-endpoints
```
</branch_commands>

## GitHub CLI Commands

<gh_commands>
```bash
# Create issue with labels
gh issue create --title "Title" --label "crate,priority:high" --milestone "v0.1.0 - MVP" --body "Description"

# List issues
gh issue list --label "priority:high"

# Create PR
gh pr create --title "feat: description" --body "Closes #123"

# Check out PR
gh pr checkout 123
```
</gh_commands>

## Issue References

<references>
- Reference issues in commits: `Refs #123`
- Close issues with: `Closes #123` or `Fixes #123`
- Link related issues in PR description
- Update issue status when starting work
</references>

## Labels

<labels>
- `epic` - Large feature/initiative
- `crate` - Rust crate implementation
- `infrastructure` - Core infrastructure
- `integration` - External service integration
- `api` - API endpoints
- `spec` - AsyncAPI spec related
- `blocked` - Blocked by another issue
- `priority:high/medium/low` - Priority levels
</labels>

## PR Guidelines

<pr_guidelines>
- Keep PRs focused and small when possible
- Include issue reference in title
- Add description of what changed and why
- Request review when ready
- Address review comments promptly
- Squash commits on merge
</pr_guidelines>
