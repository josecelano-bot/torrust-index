# Refactor plan for src/arena.rs

- Step 3.1: document internal invariants (occupied/free vector invariants).
- Step 3.2: add helper methods (len, capacity, free_slots).
- Step 3.3: add arena reuse tests + dealloc behavior tests.

## Progress
- [x] initial plan
- [x] step 3.1
- [x] step 3.2
- [x] step 3.3

## Test command
- `cargo test --lib`