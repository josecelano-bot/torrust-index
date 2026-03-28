# Refactor plan for src/tree/vtree.rs

- Step 6.1: isolate eviction candidate logic.
- Step 6.2: split large functions with helper functions.
- Step 6.3: add tests for all scan and remove leaf branches.

## Progress
- [x] initial plan
- [x] step 6.1
- [x] step 6.2
- [x] step 6.3

## Test command
- `cargo test --lib`