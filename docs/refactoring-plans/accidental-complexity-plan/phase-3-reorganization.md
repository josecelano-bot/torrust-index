# Phase 3 - Module Reorganization

## Delivery Intent

Improve cohesion and discoverability by keeping orchestrators thin and moving detailed mechanics into focused internal modules.

## Deliverables

1. Split module decomposition where orchestrator remains concise.
2. VTree internal decomposition by concern (traversal vs mutation).
3. Diagnostics organization pass for shared read-only patterns.

## Step Checklist

### P3.1 Split module decomposition

- [ ] Keep orchestrator role in split entry module
- [ ] Move detailed mechanics into focused submodules
- [ ] Ensure names describe behavior and intent

### P3.2 VTree decomposition

- [ ] Separate traversal-heavy internals from mutation-heavy internals
- [ ] Keep top-level VTree file readable as a facade
- [ ] Validate visibility boundaries to avoid leaks

### P3.3 Diagnostics organization

- [ ] Identify duplicated check/traversal patterns
- [ ] Extract shared read-only helpers where reuse is real

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
