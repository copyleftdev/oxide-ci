# Check Lingering Tickets

Audit open issues to ensure nothing is forgotten or stale.

## Steps

1. **List all open issues**
   ```bash
   gh issue list --state open --limit 50
   ```

2. **Check for stale issues** (no activity in 7+ days)
   ```bash
   gh issue list --state open --json number,title,updatedAt --jq '.[] | select(.updatedAt < (now - 604800 | todate)) | "\(.number): \(.title)"'
   ```

3. **For EACH stale issue, evaluate**
   - Is this still relevant? → Update with current status
   - Is this blocked? → Add `blocked` label and document why
   - Should this be deferred? → Add comment and close or move to backlog
   - Should this be worked on? → Prioritize and schedule

4. **Check for unassigned high-priority issues**
   ```bash
   gh issue list --label "priority:high" --state open
   ```
   - These should not sit unworked

5. **Check for issues without labels**
   ```bash
   gh issue list --state open --json number,title,labels --jq '.[] | select(.labels | length == 0) | "\(.number): \(.title)"'
   ```
   - Add appropriate labels

6. **Review blocked issues**
   ```bash
   gh issue list --label "blocked" --state open
   ```
   - For each: Is the blocker resolved?
   - If resolved: Remove `blocked` label, update status

7. **Summary report**
   - Total open issues
   - Stale issues needing attention
   - Blocked issues
   - Ready-to-work issues by priority

8. **Take action**
   - Close any tickets that are done but not closed
   - Update stale tickets with current status
   - Identify next ticket to work on
