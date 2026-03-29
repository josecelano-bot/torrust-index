# Accidental Complexity Execution Checklist (2026-03)

This checklist operationalizes the strategy in:

- [docs/accidental-complexity-elimination-plan-2026-03.md](docs/accidental-complexity-elimination-plan-2026-03.md)

Use this file as the single progress tracker.

## Status Legend

- [ ] Not started
- [-] In progress
- [x] Done
- [!] Blocked

## Execution Rules (Small, Safe Increments)

1. One micro-change per commit.
2. Keep each commit behavior-preserving.
3. Run tests after every micro-change.
4. Do not batch unrelated refactors together.
5. If a step fails tests, stop and fix before moving forward.

## Commit Size Guardrails

For each commit, target:

- 1 focused goal
- 1 to 3 files (prefer 1)
- mostly mechanical edits when moving symbols
- no simultaneous API-shape and algorithm-logic changes unless unavoidable

## Mandatory Test Gates Per Step

### Gate A (fast)

- `cargo check --all-features`

### Gate B (targeted)

Run tests for touched area only (examples):

- split flow changes: `cargo test split`
- VTree changes: `cargo test vtree`
- diagnostics changes: `cargo test diagnostics`

### Gate C (full)

- `cargo test --all-features`

A step is marked done only if Gate A + Gate B + Gate C pass.

## Step Log Template (copy for every micro-step)

### Step <id> - <title>

- Status: [ ]
- Goal:
- Files:
- Commit message:
- Risk level: Low / Medium / High
- Tests run:
  - [ ] Gate A
  - [ ] Gate B
  - [ ] Gate C
- Result notes:
- Follow-ups:

---

## Milestone 1 - Method/API Normalization

### M1.1 Inventory and classification

- [ ] Create symbol inventory of free functions receiving `&mut GvGraph`, `&GvGraph`, `&Arena<GNode<_>>`, `&Arena<VNode<_>>`.
- [ ] Classify each symbol: owner-local method / cross-tree orchestrator / pure helper.
- [ ] Add classification table at end of this checklist.

### M1.2 Convert split flow helper shape

- [ ] Convert `bootstrap_split(graph, g_id)` to private `GvGraph` method.
- [ ] Convert `catalytic_split(graph, g_id)` to private `GvGraph` method.
- [ ] Keep `attempt_split` as orchestration entrypoint on `GvGraph`.
- [ ] Update tests and call sites.

### M1.3 Convert VTree Arena-style free helpers

- [ ] Promote `propagate_v_sums` usage to `VTree::propagate_sums` at all call sites.
- [ ] Promote `sync_intensity_in_parent` usage to `VTree::sync_intensity` at all call sites.
- [ ] Promote `recompute_all_v_intensities` usage to `VTree::recompute_all_intensities` at all call sites.
- [ ] Promote `v_depth` usage to `VTree::depth` at all call sites.
- [ ] Promote `is_ancestor` usage to `VTree::is_ancestor` if method exists; otherwise add method then migrate.
- [ ] Remove redundant wrappers once all call sites are migrated.

### M1 Exit Criteria

- [ ] No newly introduced free helper takes `&mut GvGraph` unless intentionally pure orchestration entrypoint.
- [ ] Most internal callers use owner methods instead of Arena-level helpers.

---

## Milestone 2 - Introduce Missing Abstractions

### M2.1 SplitFlow phase extraction

- [ ] Extract split precondition checks into one helper (`can_split` or equivalent).
- [ ] Extract triple-parent preprocess into one helper.
- [ ] Extract child allocation + V wiring into one helper.
- [ ] Extract finalize stage (evictable propagation + plateau update + debug asserts).

### M2.2 VTree mutation mini-API

- [ ] Add/standardize child mutation methods (`replace_child`, `remove_child`, etc.).
- [ ] Add/standardize recompute+propagate method (`propagate_after_local_change` style).
- [ ] Replace repeated low-level mutation sequences with these APIs.

### M2.3 Signature simplification

- [ ] Introduce focused context structs where signatures are long (`SplitContext`, `EvictContext`), only if they reduce call-site noise.
- [ ] Ensure contexts preserve ownership clarity and do not become generic dumping structs.

### M2 Exit Criteria

- [ ] At least 3 hotspot functions have lower cognitive complexity or clearer phase structure.
- [ ] Repeated mutation/traversal snippets are replaced by named operations.

---

## Milestone 3 - Reorganization for Cohesion

### M3.1 Split module decomposition

- [ ] Keep [src/graph/algorithm/split.rs](src/graph/algorithm/split.rs) as orchestrator.
- [ ] Move detailed mechanics into focused internal modules (preconditions/phases).
- [ ] Ensure module names reflect behavior, not implementation accidents.

### M3.2 VTree module decomposition

- [ ] Separate traversal-heavy helpers from mutation-heavy helpers.
- [ ] Keep [src/tree/vtree.rs](src/tree/vtree.rs) as a readable facade.
- [ ] Avoid circular module dependencies and visibility leaks.

### M3.3 Diagnostics organization pass

- [ ] Identify duplicated traversal/check patterns in diagnostics.
- [ ] Extract shared read-only adapters/helpers where reuse is real.

### M3 Exit Criteria

- [ ] Orchestrator files are phase-readable.
- [ ] Implementation modules are cohesive and smaller.

---

## Milestone 4 - Stabilization and Proof

### M4.1 Measurement refresh

- [ ] Recompute complexity metrics with the established workflow.
- [ ] Compare before/after for top hotspot functions.
- [ ] Update [docs/complexity-analysis.md](docs/complexity-analysis.md).

### M4.2 Residual complexity map

- [ ] Document remaining complex functions as essential vs accidental.
- [ ] Create follow-up tickets for any accidental leftovers.

### M4 Exit Criteria

- [ ] No function above CC 20.
- [ ] Remaining cognitive outliers have explicit justification or next action.
- [ ] Full tests pass.

---

## Current Progress Snapshot

- Overall status: [ ] Not started
- Active milestone: [ ] M1 / [ ] M2 / [ ] M3 / [ ] M4
- Last completed step:
- Current branch:
- Last successful full test run:

## Symbol Classification Table (fill during M1.1)

| Symbol | Current location | Signature shape | Classification | Planned action | Status |
| --- | --- | --- | --- | --- | --- |
| bootstrap_split | src/graph/algorithm/split.rs | `fn(&mut GvGraph, ...)` | owner-local phase helper | convert to private GvGraph method | [ ] |
| catalytic_split | src/graph/algorithm/split.rs | `fn(&mut GvGraph, ...)` | owner-local phase helper | convert to private GvGraph method | [ ] |
| propagate_v_sums | src/tree/vtree.rs | `fn(&mut Arena<VNode<_>>, ...)` | owner behavior leaked as free function | migrate call sites to `VTree::propagate_sums` | [ ] |
| sync_intensity_in_parent | src/tree/vtree.rs | `fn(&mut Arena<VNode<_>>, ...)` | owner behavior leaked as free function | migrate call sites to `VTree::sync_intensity` | [ ] |
| recompute_all_v_intensities | src/tree/vtree.rs | `fn(&mut Arena<VNode<_>>, ...)` | owner behavior leaked as free function | migrate call sites to `VTree::recompute_all_intensities` | [ ] |
| v_depth | src/tree/vtree.rs | `fn(&Arena<VNode<_>>, ...)` | owner behavior leaked as free function | migrate call sites to `VTree::depth` | [ ] |
| is_ancestor | src/tree/vtree.rs | `fn(&Arena<VNode<_>>, ...)` | owner behavior leaked as free function | add/use `VTree::is_ancestor` then migrate | [ ] |

## Suggested PR/Commit Sequence

1. PR1: M1.1 inventory + classification only.
2. PR2: convert `bootstrap_split` + `catalytic_split` to private methods.
3. PR3: migrate first two VTree Arena-helper call-site groups.
4. PR4: finish remaining VTree helper migrations + remove wrappers.
5. PR5+: abstraction and module decomposition in small streams.
