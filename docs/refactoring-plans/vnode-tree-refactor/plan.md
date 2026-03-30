# Refactor Plan: Introduce VNodeTree as Structural Layer

## Status

In progress.

This document was cleaned and synchronized with the current implementation state on 2026-03-30.

## Goal

Introduce `VNodeTree<V>` as the structural owner of V-node topology and traversal logic while keeping `VTree<V>` as the higher-level owner of root, violations, and cross-tree orchestration.

## Resolved Decisions

1. Refactor in small, parallel-friendly slices.
2. Final direction keeps `VTree` owning `VNodeTree`.
3. Transitional adapters are allowed but must be tracked and removed later.
4. Essential tree algorithms should live in owning tree abstractions.
5. Debugging and diagnostics can remain separate helpers.
6. `GNodeTree` is explicitly out of scope for this refactor.

## Ownership Rules

Move to `VNodeTree` when logic is structural and local to V-nodes:

- parent and child navigation
- sibling lookup
- depth and ancestry traversal
- local shape predicates
- local evictable flags

Keep on `VTree` when logic depends on tree-level state or orchestration:

- `root` lifecycle
- `violations` queue lifecycle
- cross-tree flows involving both V and G trees
- top-level rebalancing workflow control

Keep as free functions only when module-local formatting or diagnostics concerns are clearer than method ownership.

## Current Shape

Implemented in code:

- `VTree.nodes` now stores `VNodeTree<V>` (wrapper over `Arena<VNode<V>>`).
- `VNodeTree` owns methods:
  - `depth`
  - `compute_has_evictable`
  - `node_has_evictable`
  - `sibling_of`
- `promote`, `rebalance`, `violation_push`, `evict`, and diagnostics helper signatures were migrated to `&VNodeTree<V>` where applicable.
- `GvGraph::vnodes()` now returns `&VNodeTree<V>`.

## Progress Tracking

### Phase Status

- [x] Phase 0: ownership model agreed
- [x] Phase 1: `VNodeTree` introduced
- [x] Phase 2: low-risk read-only helpers moved
- [x] Phase 3: violation helper boundary migrated
- [x] Phase 4: `VTree` ownership integrated
- [ ] Phase 5: cleanup and tightening

### Execution Log

| Date | Step | Status | Notes |
|---|---|---|---|
| 2026-03-30 | Introduce `VNodeTree` wrapper | done | `VTree.nodes` switched to wrapper |
| 2026-03-30 | Move structural helper methods | done | sibling/depth/evictable helpers moved |
| 2026-03-30 | Migrate helper signatures | done | rebalance, violation_push, evict, diagnostics updated |
| 2026-03-30 | Validate tests | done | `cargo test --lib --tests` passed |
| 2026-03-30 | Document cleanup | done | this plan rewritten to valid markdown |

## Transitional Adapter Debt

These are intentional temporary adapters that keep migration friction low.

| Item | Introduced | Removal Target | Notes |
|---|---|---|---|
| `Deref` / `DerefMut` from `VNodeTree` to `Arena<VNode<V>>` | Phase 1 | Phase 5 | Remove after direct arena-style access patterns are eliminated |
| `VTree` field name remains `nodes` | pre-refactor | Phase 5 | Rename to `vnodes` once churn is acceptable |

## Validation Gate

Primary gate used for this slice:

```bash
cargo test --lib --tests
```

Known caveat:

- `cargo test --all-targets` currently reaches an existing bench-path panic in dynamic tracker debug checks that is not introduced by this refactor slice.

Optional stricter gate once bench-path issue is independently resolved:

```bash
cargo test --all-targets
bash scripts/clippy-strict.sh
```

## Done Criteria for This Refactor

The refactor is considered fully finished only when all of the following are true:

1. No production helper signatures expose `&Arena<VNode<_>>`.
2. Transitional adapter debt table is empty.
3. `VTree` naming is clear (`vnodes` preferred).
4. Full project validation strategy is green for agreed CI targets.
5. This plan is updated with final outcomes and no pending cleanup items.

## Remaining Work

1. Remove transitional `Deref`/`DerefMut` usage by replacing implicit arena-style access with explicit `VNodeTree` API.
2. Rename `VTree.nodes` to `VTree.vnodes` when the remaining call-site churn is acceptable.
3. Audit test-only construction sites that still instantiate raw `Arena<VNode<_>>` and decide whether they should stay as fixture convenience or move to `VNodeTree` constructors.
4. Run strict lint gate and record outcomes.

## Out of Scope

- Introducing `GNodeTree`.
- Reworking plateau tracker architecture.
- Altering rebalance semantics.

