# Create GitHub Issue

Workflow for creating a well-structured GitHub issue.

## Steps

1. **Determine issue type and labels**
   - `crate` - Rust crate implementation
   - `infrastructure` - Core infrastructure
   - `integration` - External service integration
   - `api` - API endpoints
   - `spec` - AsyncAPI spec related
   - Priority: `priority:high`, `priority:medium`, `priority:low`

2. **Check for existing issues**
   ```bash
   gh issue list --search "keyword"
   ```

3. **Gather context**
   - What problem does this solve?
   - What are the acceptance criteria?
   - What are the dependencies?
   - What spec schemas are involved?

4. **Create the issue** using this template:
   ```bash
   gh issue create \
     --title "[Type] Short descriptive title" \
     --label "crate,priority:medium" \
     --milestone "v0.1.0 - MVP" \
     --body '## Summary
   Brief description of what needs to be done.

   ## Remaining Work
   - [ ] Task 1
   - [ ] Task 2
   - [ ] Task 3

   ## Acceptance Criteria
   \`\`\`rust
   // Code example showing expected behavior
   \`\`\`

   ## Spec References
   - `spec/schemas/xxx.yaml`

   ## Dependencies
   - `oxide-core` for domain types

   ## Blocked By
   - #XX if applicable
   '
   ```

5. **Verify issue created**
   ```bash
   gh issue view --web
   ```

6. **Link related issues** if applicable
   - Add comment referencing related issues
   - Update parent epic checklist
