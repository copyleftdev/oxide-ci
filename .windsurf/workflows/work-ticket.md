# Work Ticket

Complete workflow for picking up, working through, and closing a GitHub issue.

## Steps

1. **Select a ticket to work on**
   ```bash
   gh issue list --label "priority:high" --state open
   ```
   - Choose the highest priority unblocked issue
   - Or specify issue number if you know it

2. **Review the issue thoroughly**
   ```bash
   gh issue view [ISSUE_NUMBER]
   ```
   - Read the summary and requirements
   - Understand all acceptance criteria
   - Check for dependencies or blockers
   - Note any spec references

3. **Create a feature branch**
   ```bash
   git checkout main
   git pull origin main
   git checkout -b feat/issue-[NUMBER]-[short-description]
   ```
   - Use `feat/` for features, `fix/` for bugs
   - Keep description short but meaningful

4. **Add a comment that you're starting**
   ```bash
   gh issue comment [ISSUE_NUMBER] --body "🚀 Starting work on this issue"
   ```

5. **Implement the work**
   - Follow the acceptance criteria step by step
   - Make atomic commits with issue references
   - Run tests frequently: `cargo test -p [crate]`

6. **For EACH acceptance criterion**
   - Implement the requirement
   - Write/update tests
   - Verify it works
   - Commit: `git commit -m "feat(scope): implement X - Refs #[NUMBER]"`

7. **Final validation checklist**
   ```bash
   # Run all checks
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   
   # If spec changes
   make lint
   ```

8. **Merge to main**
   ```bash
   git checkout main
   git pull origin main
   git merge feat/issue-[NUMBER]-[short-description]
   git push origin main
   ```

9. **Clean up branch**
   ```bash
   git branch -d feat/issue-[NUMBER]-[short-description]
   git push origin --delete feat/issue-[NUMBER]-[short-description] 2>/dev/null || true
   ```

10. **Close the issue with summary**
    ```bash
    gh issue close [ISSUE_NUMBER] --comment "✅ Completed

    ## Summary
    - [Brief description of what was implemented]
    
    ## Commits
    - [List key commits]
    
    ## Verification
    - All acceptance criteria met
    - Tests passing
    - Merged to main"
    ```

11. **Verify closure**
    ```bash
    gh issue view [ISSUE_NUMBER] --json state
    ```
    - Confirm state is "CLOSED"
