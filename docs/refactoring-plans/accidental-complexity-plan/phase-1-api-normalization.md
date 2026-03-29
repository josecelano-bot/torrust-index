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

- [x] List symbols taking `&mut GvGraph`, `&GvGraph`, `&Arena<GNode<_>>`, `&Arena<VNode<_>>`
- [x] Classify each symbol
- [x] Record migration decision per symbol

### P1.2 Split helper method conversion

- [x] Convert `bootstrap_split(graph, g_id)` -> private `self.bootstrap_split(g_id)`
- [x] Convert `catalytic_split(graph, g_id)` -> private `self.catalytic_split(g_id)`
- [x] Keep `attempt_split` as orchestrator entrypoint
- [x] Update call sites

### P1.3 VTree call-site migration

- [x] Migrate to `VTree::propagate_sums`
- [x] Migrate to `VTree::sync_intensity`
- [x] Migrate to `VTree::recompute_all_intensities`
- [x] Migrate to `VTree::depth`
- [x] Migrate to `VTree::is_ancestor` or add and then migrate
- [x] Remove wrappers after all call sites are migrated

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

- Step: P1.3 VTree call-site migration (partial)
- Status: [x]
- Files: `src/tree/vtree.rs`, `src/diagnostics/diagnostic.rs`, `src/diagnostics/diagnostic/diagnose.rs`, `src/diagnostics/diagnostic/logging.rs`, `src/graph/algorithm/evict.rs`
- Commit: [created]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Added `VTree::is_ancestor` and routed eviction-time missed-violation diagnostics through an in-tree owner API.
- Follow-up: Continue reducing direct arena helper exposure where owner APIs are practical.

### 2026-03-29

- Step: P1.3 wrapper exposure cleanup (partial)
- Status: [x]
- Files: `src/tree/vtree.rs`
- Commit: `ebbee4a`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Tightened visibility of migrated arena-first helpers (`propagate_v_sums`, `recompute_all_v_intensities`, `sync_intensity_in_parent`) and related internal utilities to keep owner-method paths as the primary API surface.
- Follow-up: Complete the remaining `Remove wrappers after all call sites are migrated` checklist item after depth/ancestry helper exposure is fully reconciled.

### 2026-03-29

- Step: P1.3 wrapper exposure cleanup (vtree leaf removal)
- Status: [x]
- Files: `src/tree/vtree.rs`
- Commit: `a01d354`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: `vtree_remove_leaf` is now module-internal since all external use is already routed through `VTree::remove_leaf`.
- Follow-up: Continue reducing remaining non-owner helper exposure where call-site migration is complete.

### 2026-03-29

- Step: P1.3 wrapper exposure cleanup (depth/ancestry wrappers)
- Status: [x]
- Files: `src/tree/vtree.rs`, `src/graph/algorithm/rebalance/resolve.rs`, `src/graph/algorithm/rebalance/violation_scan.rs`, `src/diagnostics/diagnostic/logging.rs`
- Commit: `6e20453`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Removed external reliance on free `v_depth`/`is_ancestor` wrappers by localizing helper logic to consuming modules and keeping wrapper implementations internal to `vtree`.
- Follow-up: P1.3 migration checklist is now complete; keep owner-method APIs as the primary integration boundary.

### 2026-03-29

- Step: P1.1 inventory and classification
- Status: [x]
- Files: `docs/refactoring-plans/accidental-complexity-plan/phase-1-inventory-2026-03-29.md`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Inventory completed and classified by owner-local, helper, orchestrator, and trait-boundary roles.
- Follow-up: Continue P1.3 call-site migration where an owner API is clearly available.

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
