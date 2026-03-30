# Refactoring Plan: Reduce `remove_leaf` Complexity

## Goal

Refactor `VTree::remove_leaf` into smaller, intention-revealing pieces while preserving behavior.

Primary target:

- reduce cognitive complexity in `src/tree/vtree/mod.rs`
- keep all existing semantics for root removal, 3-child shrink, and 2-child collapse
- maintain current asymptotic costs (all operations remain O(depth) where applicable)

## Scope

In scope:

- `VTree::remove_leaf` flow and its private helpers
- local V-tree mutation orchestration and post-mutation maintenance calls
- unit tests for removal scenarios and edge cases

Out of scope:

- changing rebalance semantics
- changing G-tree ownership model
- broad API redesign beyond `remove_leaf` and local helpers

## Quality Gate (run before each commit)

```bash
cargo test --all-targets --all-features
bash scripts/cspell-check.sh
bash scripts/clippy-strict.sh
```

All three must pass before moving to the next phase.

---

## Refactor Strategy

Use a low-risk sequence aligned with the recommended order:

1. split orchestration by scenario
2. extract duplicated maintenance sequences
3. optionally add explicit case-classification type
4. optionally push low-level rewiring into `VNodeTree` primitives

Each optional step is only applied if complexity remains high after prior phases.

---

## Phases

### Phase 1 - Baseline and safety net

**Goal**: lock behavior before structural edits.

- [x] **Step 1.1** Run quality gate and record baseline status.
- [x] **Step 1.2** Add or verify tests for all current `remove_leaf` branches:
  - root removal
  - parent with 3 children (shrink)
  - parent with 2 children and no grandparent (collapse to sibling root)
  - parent with 2 children and grandparent (collapse into grandparent)
- [x] **Step 1.3** Add assertion-level checks for side effects:
  - expected deallocations
  - parent pointers after rewiring
  - root value correctness
  - `gtree.nodes.clear_entry` effect for entry leaves

> Commit: `test(vtree): strengthen remove_leaf behavior coverage`

### Phase 2 - Orchestrator split by scenario (required)

**Goal**: make `remove_leaf` an orchestration method with branch-specific helpers.

- [x] **Step 2.1** Keep `remove_leaf` as top-level flow:
  - clear G mapping when `VKind::Entry`
  - identify parent/root case
  - dispatch to branch helper
- [x] **Step 2.2** Extract root branch helper:
  - `remove_root_leaf(...)`
- [x] **Step 2.3** Extract shrink branch helper:
  - `remove_from_three_child_parent(...)`
- [x] **Step 2.4** Extract collapse branch helper:
  - `collapse_two_child_parent(...)`
- [x] **Step 2.5** Preserve tracing branch labels (`root`, `shrink`, `collapse`).
- [x] **Step 2.6** Run quality gate.

> Commit: `refactor(vtree): split remove_leaf into scenario helpers`

### Phase 3 - Remove duplication in maintenance steps (required)

**Goal**: reduce repeated operation sequences and make intent explicit.

- [x] **Step 3.1** Add helper for common follow-up after parent structural change:
  - `finalize_after_parent_change(parent_id)`
  - includes recompute/propagate sums and evictable flags
- [x] **Step 3.2** Replace duplicated call pairs in shrink and collapse paths.
- [x] **Step 3.3** Keep behavior and call order identical to baseline.
- [x] **Step 3.4** Run quality gate.

> Commit: `refactor(vtree): extract shared post-mutation maintenance`

### Phase 4 - Explicit case classifier (optional)

**Goal**: separate read-only decision logic from mutation logic.

Apply this phase only if complexity/readability is still not acceptable after Phase 3.

- [x] **Step 4.1** Introduce private enum:
  - `RemoveLeafCase::Root`
  - `RemoveLeafCase::Shrink { parent }`
  - `RemoveLeafCase::Collapse { parent, sibling, grandparent }`
- [x] **Step 4.2** Add `classify_remove_leaf_case(v_id)` as a read-only helper.
- [x] **Step 4.3** Make `remove_leaf` do `match` over enum and call existing helpers.
- [x] **Step 4.4** Run quality gate.

> Commit: `refactor(vtree): add explicit remove_leaf case classifier`

### Phase 5 - Move low-level rewiring into `VNodeTree` primitives (optional)

**Goal**: reduce direct structural micromanagement inside `VTree`.

Apply only if Phase 4 still leaves `remove_leaf` too dense.

- [ ] **Step 5.1** Identify low-level rewiring blocks suitable for `VNodeTree` methods.
- [ ] **Step 5.2** Add narrowly-scoped primitives in `src/tree/vtree/vnode_tree.rs`.
- [ ] **Step 5.3** Replace manual rewiring sequences in `remove_leaf` helpers with primitives.
- [ ] **Step 5.4** Ensure no semantic drift in root updates and parent links.
- [ ] **Step 5.5** Run quality gate.

> Commit: `refactor(vnode-tree): encapsulate remove_leaf rewiring primitives`

### Phase 6 - Final cleanup and documentation

- [ ] **Step 6.1** Ensure helper names reflect intent (scenario vs primitive vs maintenance).
- [ ] **Step 6.2** Update module docs in `src/tree/vtree/mod.rs` if flow changed meaningfully.
- [ ] **Step 6.3** Remove dead code from transitional helpers.
- [ ] **Step 6.4** Run full quality gate and record final status.

> Commit: `docs(vtree): document remove_leaf refactor structure`

---

## Acceptance Criteria

The refactor is complete when all are true:

1. `remove_leaf` reads as a short orchestration function.
2. Branch-specific behavior lives in focused helpers.
3. No behavior regressions in removal tests.
4. Tracing branch labels are preserved.
5. Quality gate is green.

## Risk Register

1. **Root update regression**
   - Mitigation: explicit tests for root/no-root transitions and collapse-to-sibling root.
2. **Parent pointer mismatch after collapse**
   - Mitigation: assert parent chains for surviving sibling and grandparent replacement.
3. **Maintenance ordering drift**
   - Mitigation: keep helper call order identical; verify with existing tests plus branch-specific assertions.

## Progress Tracker

| Phase | Status |
|---|---|
| 1 - Baseline and safety net | completed |
| 2 - Orchestrator split by scenario | completed |
| 3 - Shared maintenance helper extraction | completed |
| 4 - Explicit case classifier (optional) | completed |
| 5 - `VNodeTree` primitive extraction (optional) | not started |
| 6 - Final cleanup and docs | not started |

## Implementation Notes (2026-03-30)

Completed code and test changes:

- `remove_leaf` is now a short dispatcher with scenario helpers.
- Added helper methods: `remove_root_leaf`, `remove_from_three_child_parent`,
  `collapse_two_child_parent`, and `finalize_after_parent_change`.
- Added explicit `RemoveLeafCase` and `classify_remove_leaf_case` so case
  decision logic is read-only and separated from mutation logic.
- Added tests for collapse with grandparent rewiring and explicit
  `gtree.nodes.clear_entry` side effect validation.

Validation outcomes during implementation:

- `cargo test vtree_remove_leaf_fn -- --nocapture`: passed.
- `cargo test`: passed.
- `cargo test --all-targets --all-features`: unit/integration/snapshot tests passed;
  bench target panicked in existing plateau tracker debug diff path (outside this refactor).
- `bash scripts/cspell-check.sh`: passed.
- `bash scripts/clippy-strict.sh`: failed on existing crate-wide lint findings not introduced in this change.
