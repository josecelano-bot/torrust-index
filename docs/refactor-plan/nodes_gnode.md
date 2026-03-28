# Refactor plan for src/nodes/gnode.rs

- Step 4.1: add `is_leaf`, `has_children`, `child_ids` helpers.
- Step 4.2: add `validate` method with debug invariant checks.
- Step 4.3: add unit tests covering state transitions.

## Progress
- [x] initial plan
- [ ] step 4.1
- [ ] step 4.2
- [ ] step 4.3

## Test command
- `cargo test --lib`