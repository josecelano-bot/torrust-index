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

- [ ] Convert `bootstrap_split(graph, g_id)` -> private `self.bootstrap_split(g_id)`
- [ ] Convert `catalytic_split(graph, g_id)` -> private `self.catalytic_split(g_id)`
- [ ] Keep `attempt_split` as orchestrator entrypoint
- [ ] Update call sites

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
