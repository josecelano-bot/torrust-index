# Phase 3 - Module Reorganization

## Delivery Intent

Improve cohesion and discoverability by keeping orchestrators thin and moving detailed mechanics into focused internal modules.

## Deliverables

1. Split module decomposition where orchestrator remains concise.
2. VTree internal decomposition by concern (traversal vs mutation).
3. Diagnostics organization pass for shared read-only patterns.

## Step Checklist

### P3.1 Split module decomposition

- [x] Keep orchestrator role in split entry module
- [x] Move detailed mechanics into focused submodules
- [x] Ensure names describe behavior and intent

### P3.2 VTree decomposition

- [x] Separate traversal-heavy internals from mutation-heavy internals
- [x] Keep top-level VTree file readable as a facade
- [x] Validate visibility boundaries to avoid leaks

### P3.3 Diagnostics organization

- [x] Identify duplicated check/traversal patterns
- [x] Extract shared read-only helpers where reuse is real

## Acceptance Criteria

- [ ] Orchestrator files are shorter and phase-readable
- [ ] Internal modules have single clear responsibility
- [ ] No circular dependencies introduced
- [ ] No behavior changes

## Test Checklist (per step)

- [ ] Gate A: `cargo check --all-features`
- [ ] Gate B: targeted tests for touched area
- [ ] Gate C: `cargo test --all-features`

## Progress Log

### Step Template

- Step:
- Status: [ ]
- Files:
- Commit:
- Tests:
  - [ ] Gate A
  - [ ] Gate B
  - [ ] Gate C
- Notes:
- Follow-up:

### 2026-03-29

- Step: P3.1 split module decomposition
- Status: [x]
- Files: `src/graph/algorithm/split.rs`, `src/graph/algorithm/split/helpers.rs`
- Commit: `dc2aed7`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Kept `split.rs` as the orchestration facade and moved candidate/preprocess/allocation/mirror mechanics into `split/helpers.rs` with focused helper names.
- Follow-up: Continue P3.2 by separating traversal-heavy and mutation-heavy internals in `src/tree/vtree.rs`.

### 2026-03-29

- Step: P3.2 VTree decomposition (traversal extraction)
- Status: [ ]
- Files: `src/tree/vtree.rs`, `src/tree/vtree/traversal.rs`
- Commit: `2bcd99b`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Extracted traversal-focused internals (`v_depth`, strict-ancestor walk, evictable-child predicate) from the VTree facade into `tree/vtree/traversal.rs`.
- Follow-up: Continue P3.2 by extracting mutation-heavy internals into a dedicated companion module.

### 2026-03-29

- Step: P3.2 VTree decomposition (mutation extraction completion)
- Status: [x]
- Files: `src/tree/vtree.rs`, `src/tree/vtree/mutation.rs`
- Commit: `bfe44b1`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Extracted structural-mutation helpers (add/replace/remove child, entry flags, evictable setter) into `tree/vtree/mutation.rs` and kept crate-visible API compatibility through controlled re-exports.
- Follow-up: Continue P3.3 diagnostics organization by consolidating duplicated read-only traversal/check patterns.

### 2026-03-29

- Step: P3.3 diagnostics organization (collapse-sibling dedup)
- Status: [ ]
- Files: `src/diagnostics/diagnostic/logging.rs`
- Commit: `962bfda`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Removed duplicated in-tree collapse-sibling diagnostic body and routed it through the shared arena-based implementation.
- Follow-up: Continue P3.3 by extracting additional read-only helpers only where reuse is clear.

### 2026-03-29

- Step: P3.3 diagnostics organization (eviction audit dedup)
- Status: [x]
- Files: `src/graph/algorithm/evict.rs`
- Commit: `6b115b0`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Replaced duplicated queue-diff logic in eviction diagnostics with shared `diagnostics::audit_violations` helper and retained context-specific diagnosis on missed nodes.
- Follow-up: Reorganization phase steps are now complete; continue with stabilization/metrics tasks.
