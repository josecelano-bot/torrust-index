# Phase 1 - API Normalization

## Delivery Intent

Make ownership boundaries explicit by converting method-shaped free functions into owner methods and reducing Arena leakage at call sites.

## Deliverables

1. Symbol inventory for method-shaped free functions.
2. Classification map: owner-local method, cross-tree orchestrator, pure helper.
3. Split helper conversion:
   - `bootstrap_split` to private `GvGraph` method
   - `catalytic_split` to private `GvGraph` method
4. VTree call-site migration away from Arena-first helper calls.

## Step Checklist

### P1.1 Inventory and classification

- [ ] List symbols taking `&mut GvGraph`, `&GvGraph`, `&Arena<GNode<_>>`, `&Arena<VNode<_>>`
- [ ] Classify each symbol
- [ ] Record migration decision per symbol

### P1.2 Split helper method conversion

- [x] Convert `bootstrap_split(graph, g_id)` -> private `self.bootstrap_split(g_id)`
- [x] Convert `catalytic_split(graph, g_id)` -> private `self.catalytic_split(g_id)`
- [x] Keep `attempt_split` as orchestrator entrypoint
- [x] Update call sites

### P1.3 VTree call-site migration

- [ ] Migrate to `VTree::propagate_sums`
- [ ] Migrate to `VTree::sync_intensity`
- [ ] Migrate to `VTree::recompute_all_intensities`
- [ ] Migrate to `VTree::depth`
- [ ] Migrate to `VTree::is_ancestor` or add and then migrate
- [ ] Remove wrappers after all call sites are migrated

## Acceptance Criteria

- [ ] No new method-shaped free functions introduced
- [ ] Owner methods are used at most internal call sites
- [ ] No behavior changes

## Test Checklist (per step)

- [x] Gate A: `cargo check --all-features`
- [x] Gate B: targeted tests for touched area
- [x] Gate C: `cargo test --all-features`

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

- Step: P1.2 split helper method conversion
- Status: [x]
- Files: `src/graph/algorithm/split.rs`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: `bootstrap_split` and `catalytic_split` now live on `GvGraph`; `attempt_split` remains the entrypoint.
- Follow-up: Complete P1.1 inventory and P1.3 VTree migration before calling Phase 1 complete.
