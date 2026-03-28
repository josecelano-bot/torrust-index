# Refactor plan for src/handle.rs

- Step 2.1: add constructor invariants and docs.
- Step 2.2: add TryFrom<u32> and unit tests.
- Step 2.3: assert check for non-zero and split ID roundtrip.

## Progress
- [x] initial plan
- [ ] step 2.1
- [ ] step 2.2
- [ ] step 2.3

## Test command
- `cargo test --lib`