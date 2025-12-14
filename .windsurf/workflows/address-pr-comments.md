# Address PR Comments

Workflow for addressing pull request review comments.

## Steps

1. **Check out the PR branch**
   ```bash
   gh pr checkout [PR_NUMBER]
   ```

2. **Get comments on PR**
   ```bash
   gh api --paginate repos/copyleftdev/oxide-ci/pulls/[PR_NUMBER]/comments | jq '.[] | {user: .user.login, body, path, line, created_at}'
   ```

3. **For EACH comment, do the following**
   Address one comment at a time:
   
   a. Print out: "(index). From [user] on [file]:[line] — [body]"
   
   b. Read and analyze the file and the line range mentioned
   
   c. If you don't understand the comment:
      - Ask for clarification
      - Or note it needs manual attention
   
   d. If you can address the comment:
      - Make the code change
      - Verify the change compiles: `cargo check`
      - Move to the next comment

4. **Run tests after all changes**
   ```bash
   cargo test --workspace
   cargo clippy --workspace
   ```

5. **Commit the changes**
   ```bash
   git add -A
   git commit -m "refactor: address PR review comments"
   git push
   ```

6. **Summarize**
   - List comments that were addressed
   - List comments that need user attention
   - Note any questions for the reviewer
