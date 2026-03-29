# Phase 1 Inventory and Classification (2026-03-29)

Purpose: satisfy P1.1 by listing method-shaped free functions and classifying
each as owner-local method, cross-tree orchestrator, pure helper, or
trait-boundary API.

## Summary

- `&mut GvGraph` free functions: no production functions remain.
- `&GvGraph` functions: diagnostics/read-only facades (kept as read APIs).
- `&Arena<GNode<_>>` and `&Arena<VNode<_>>` signatures remain where they are
  local algorithms, trait contracts, or module-private mechanics.

## Classification Map

| Symbol Group | Representative Symbols | Current Shape | Classification | Decision |
|---|---|---|---|---|
| Split entrypoints | `attempt_split`, `bootstrap_split`, `catalytic_split` in `src/graph/algorithm/split.rs` | Owner methods on `GvGraph` | Owner-local method | Keep as methods (P1.2 done) |
| VTree owner wrappers | `VTree::sync_intensity`, `VTree::propagate_sums`, `VTree::recompute_all_intensities`, `VTree::depth` | Owner methods on `VTree` | Owner-local method | Keep and continue using |
| Rebalance core (V arena) | `resolve`, `contract`, `standard_promote`, `skip_promote`, `legacy_promote` | Free functions over `&mut Arena<VNode<_>>` | Cross-tree orchestrator / local algorithm | Keep free for now; Phase 2 context/APIs reduce parameter noise |
| Violation push helpers | `push_*_violations` in `src/graph/algorithm/violation_push.rs` | Free functions over `&Arena<VNode<_>>` | Pure helper | Keep free helper style |
| Plateau tracker contract | `PlateauTracking::*` trait in `src/traits/plateau_tracking.rs` | Trait methods with `&Arena<GNode<_>>` | Trait-boundary API | Keep signature shape (intentional boundary) |
| Dynamic tracker internals | `on_*_impl`, `normalize_impl`, `repair_p_i4_impl` | Tracker-owned methods, still receive `gnodes` | Owner-local method with explicit dependency | Keep; context wrapper is optional future optimization |
| Diagnostics over graph | `dump_*`, invariant checks, plateau audits taking `&GvGraph` | Read-only APIs | Read facade | Keep as read-only diagnostic entrypoints |
| Ancestry/depth helpers | `is_ancestor`, `v_depth` free functions in `src/tree/vtree.rs` | Free functions + some method wrappers | Transitional helper | Candidate for later consolidation, not Phase 1 blocker |

## Migration Decisions Recorded

1. Completed in Phase 1: method-shaped split helpers are methods on `GvGraph`.
2. Deferred beyond Phase 1: deep rebalance/promote internals continue to use
   arena-local helpers until abstraction phase work completes.
3. No behavior changes introduced by P1.1/P1.2 inventory-based decisions.
