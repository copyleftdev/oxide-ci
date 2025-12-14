---
trigger: always_on
---

# Solo Developer GitHub Workflow

Treat GitHub Issues as if working with a professional development team. Every piece of work flows through a ticket.

## Issue Lifecycle

<lifecycle>
1. **Pick up** — Assign yourself, move to "In Progress"
2. **Branch** — Create feature branch from `main`
3. **Implement** — Complete all acceptance criteria
4. **Validate** — Verify each acceptance criterion is met
5. **Close** — Commit with issue reference, close the ticket
</lifecycle>

## Core Principles

<principles>
- **No work without a ticket** — Every change traces to an issue
- **No lingering tickets** — Complete or explicitly defer; never leave in limbo
- **One ticket at a time** — Focus on completing current work before starting new
- **Acceptance criteria are law** — Don't close until ALL criteria are verified
- **Branch per ticket** — Isolate work for clean history
</principles>

## Branch Management

<branching>
- Branch naming: `feat/issue-{number}-{short-description}` or `fix/issue-{number}-{short-description}`
- Always branch from latest `main`
- Merge directly to `main` when complete (no PR needed for solo dev)
- Delete branch after merge
- Keep `main` always deployable
</branching>

## Commit Standards

<commits>
- Reference issue in every commit: `Refs #123` or `Closes #123`
- Use `Closes #123` in final commit to auto-close the issue
- Conventional commit format: `feat(scope): description`
- Atomic commits — one logical change per commit
</commits>

## Before Closing a Ticket

<checklist>
- [ ] All acceptance criteria verified
- [ ] Tests pass: `cargo test --workspace`
- [ ] Lints pass: `cargo clippy --workspace`
- [ ] Code formatted: `cargo fmt`
- [ ] Spec validated (if applicable): `make lint`
- [ ] Branch merged to `main`
- [ ] Branch deleted
- [ ] Issue closed with summary comment
</checklist>

## Handling Blocked Work

<blocked>
- Add `blocked` label immediately when blocked
- Document blocker in issue comment
- Move to next available ticket
- Return when blocker is resolved
- Never leave blocked tickets without documentation
</blocked>
