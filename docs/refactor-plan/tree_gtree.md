# Refactor plan for src/tree/gtree.rs

- Step 5.1: split compute functions into small private helpers.
- Step 5.2: add boundary tests for `route_to` and `allocate_children`.
- Step 5.3: add docs for depth and headroom invariants.

## Progress
- [x] initial plan
- [ ] step 5.1
- [ ] step 5.2
- [ ] step 5.3

## Test command
- `cargo test --lib`