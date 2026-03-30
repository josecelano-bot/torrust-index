# Refactor Plan: Introduce GNodeTree as Structural Layer

## Status

**Completed.**

All five phases have been implemented and committed.  The full test gate
(515 lib + 10 doc + 4 integration = 529 tests) is green on every phase commit.

## Goal

Introduce `GNodeTree<C, V>` as the structural owner of G-node topology,
traversal logic, root identity, and intrinsic structural statistics, while
keeping `GTree<C, V, N>` as the higher-level owner of policy parameters and
cross-tree orchestration.

The layering after this refactor:

- `Arena<T>` remains storage-only.
- `GNodeTree<C, V>` owns everything whose correctness depends only on the node
  set: the backing store, the root, the live node count, and the terminal count.
- `GTree<C, V, N>` wraps `GNodeTree` and adds the policy constraints: bit-width
  constant `N`, depth limits, headroom, and soft limit.

## Resolved Decisions

1. This is a new plan, not a rewrite of `gtree-tda-refactor`.
2. The structural wrapper is named `GNodeTree<C, V>` with no const generic.
   Depth methods that need the bit-width take `n: u32` as an explicit parameter,
   consistent with the existing free functions.
3. Refactor in small slices with temporary adapters allowed, but track them.
4. `GNodeTree<C, V>` owns `nodes`, `root`, `node_count`, and `terminal_count`.
   These four fields together define what a G-node tree *is*; they do not depend
   on any policy parameter.  Everything else stays on `GTree`.
5. `GTree<C, V, N>` keeps `live_depth_evict`, `live_depth_create`,
   `depth_buffer`, `headroom`, and `soft_limit`.  These constrain which trees
   are valid in the domain context.
6. Sum recomputation helpers (`recompute_sums`, `recompute_sums_subtree`) move
   to `GNodeTree`; they only touch node data and carry no policy dependency.
7. Plateau tracking consumes `&GNodeTree<C, V>` directly.  No narrower `GTree`
   facade is introduced unless a future tracker method needs policy access.
8. The `GTree` field holding `GNodeTree` keeps the name `nodes` during
   migration.  A rename is deferred to Phase 5.
9. Plateau tracking is part of the migration surface because it currently
   exposes the raw read-only G-node arena broadly.
10. This plan focuses on replacing read-only `&Arena<GNode<C, V>>` boundaries;
    the earlier mutable-boundary cleanup is already complete.

## Ownership Rules

**Primary rule**: move to `GNodeTree` when the logic depends only on fields
that live inside `GNodeTree` itself (`nodes`, `root`, `node_count`,
`terminal_count`).  A method belongs on `GNodeTree` if removing *all* policy
fields from `GTree` would not require changing that method.

Move to `GNodeTree`:

- backing store access — `get`, `get_mut`, `alloc`, `dealloc`, `is_occupied`
- root identity and root-anchored traversal (`route_to`)
- node and terminal counters (`node_count`, `terminal_count`) and mutations
  that maintain them
- parent/child/sibling navigation
- subtree shape queries
- local depth queries using an explicit `n` parameter
- uniform contour traversal
- structural link, detach, and merge primitives
- sum recomputation (`recompute_sums`, `recompute_sums_subtree`)

Keep on `GTree`:

- policy parameters: `live_depth_evict`, `live_depth_create`, `depth_buffer`,
  `headroom`, `soft_limit`
- derived policy helpers such as `depth_of_interval` (which bakes in `N`)
- `uniform_contour_depth` as a `GTree` convenience method that supplies `N`
  from the const generic; the underlying traversal lives on `GNodeTree`
- operations that check policy (budget exceeded, soft limit, eviction trigger)
- cross-tree workflows involving both G and V trees
- tracker lifecycle owned by `GvGraph`

Keep as free functions:

- pure domain math with no node-container dependency: `gnode_depth_from_interval`,
  `gnode_depth_from_range`

## Current Shape

Current code still uses the raw arena as the G-node structural surface:

- `GTree.nodes` stores `Arena<GNode<C, V>>`.
- `GTree` methods such as `recompute_sums`, `recompute_sums_subtree`,
  `allocate_children`, `allocate_missing_child`, `assign_entry`, `clear_entry`,
  and `merge_into_parent` operate directly on the arena.
- `uniform_contour_depth_of` in `src/tree/gtree.rs` is still a free function
  taking `&Arena<GNode<C, V>>`.
- The plateau tracking trait and both tracker implementations expose many
  `gnodes: &Arena<GNode<C, V>>` read-only signatures.

## Proposed End State

### `GNodeTree<C, V>` fields

```rust
pub struct GNodeTree<C: Coordinate, V: Accumulator> {
    nodes: Arena<GNode<C, V>>,
    root: GNodeId,
    node_count: u32,
    terminal_count: u32,
}
```

### `GTree<C, V, N>` fields after migration

```rust
pub struct GTree<C: Coordinate, V: Accumulator, const N: u32> {
    nodes: GNodeTree<C, V>,       // name kept during migration
    live_depth_evict: u32,
    live_depth_create: u32,
    depth_buffer: u32,
    headroom: usize,
    soft_limit: Option<usize>,
}
```

### Method distribution

`GNodeTree` methods (representative list; not exhaustive):

- `route_to`
- `recompute_sums`, `recompute_sums_subtree`
- `uniform_contour_depth_of` (takes `n: u32`)
- `allocate_children`, `allocate_missing_child`
- `assign_entry`, `clear_entry`
- `merge_into_parent`
- parent/child/sibling traversal helpers

`GTree` retains:

- `depth_of_interval` (bakes in `N`)
- `uniform_contour_depth` (convenience wrapper supplying `N`)
- policy predicates and budget checks
- any cross-tree workflow that needs both G and V data

Plateau tracking trait and implementations accept `&GNodeTree<C, V>` instead of
`&Arena<GNode<C, V>>`.

## Progress Tracking

### Phase Status

- [x] Phase 0: boundary and scope reviewed
- [x] Phase 1: introduce `GNodeTree` wrapper
- [x] Phase 2: move low-risk structural helpers
- [x] Phase 3: migrate plateau tracking read-only boundary
- [x] Phase 4: remove transitional Deref adapters
- [x] Phase 5: cleanup and tightening

### Execution Log

| Date | Step | Status | Notes |
|---|---|---|---|
| 2026-03-30 | Review current G-side raw arena boundary | done | Confirmed `GTree` still owns raw arena and `uniform_contour_depth_of` is still free |
| 2026-03-30 | Inventory `gnodes: &Arena<GNode<C, V>>` signatures | done | 61 exact-signature matches across G-tree and plateau tracking code |
| 2026-03-30 | Write `GNodeTree` plan | done | Initial plan added |
| 2026-04 | Phase 1: Introduce `GNodeTree` wrapper | done | Commit `7cfea13` — 4 borrow conflicts fixed; all 529 tests green |
| 2026-04 | Phase 2: Move structural helpers to `GNodeTree` | done | Commit `f127f6c` — `route_to`, `recompute_sums*`, allocation/link helpers moved |
| 2026-04 | Phase 3: Migrate plateau tracking boundary | done | Commit `e286851` — all `gnodes: &Arena` signatures replaced with `&GNodeTree` |
| 2026-04 | Phase 4: Remove transitional Deref adapters | done | Commit `e0bf988` — both `GNodeTree→Arena` and `GTree→GNodeTree` Deref removed; delegation methods added; 529 tests green |
| 2026-04 | Phase 5: Cleanup and plan update | done | This commit — plan updated, all done criteria verified |

## Migration Inventory

Exact current matches for `gnodes: &Arena<GNode<C, V>>`: 61.

### `src/tree/gtree.rs`

- `uniform_contour_depth_of`

### `src/traits/plateau_tracking.rs`

- `on_observe`
- `on_bootstrap_split`
- `on_catalytic_split`
- `on_evict`
- `on_legacy_promotes_batched`
- `normalize`
- `repair_p_i4`
- `recompute_sums`
- `debug_assert_mirror_consistency`

### `src/graph/algorithm/plateau/dynamic_tracker.rs`

- `on_observe`
- `on_bootstrap_split`
- `on_catalytic_split`
- `on_evict`
- `on_legacy_promotes_batched`
- `normalize`
- `repair_p_i4`
- `recompute_sums`
- `debug_assert_mirror_consistency`

### `src/graph/algorithm/plateau/noop_tracker.rs`

- `on_observe`
- `on_bootstrap_split`
- `on_catalytic_split`
- `on_evict`
- `on_legacy_promotes_batched`
- `normalize`
- `repair_p_i4`
- `recompute_sums`

### `src/graph/algorithm/plateau/dynamic_tracker/observe.rs`

- `on_observe_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/legacy_promotes.rs`

- `on_legacy_promotes_batched_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/repair.rs`

- `repair_p_i4_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/normalize.rs`

- `normalize_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/evict.rs`

- `on_evict_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/split_bootstrap.rs`

- `on_bootstrap_split_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/split_catalytic.rs`

- `catalytic_split_depths`
- `collect_path_siblings_for_split`
- `locate_covering_basis_for_catalytic_split`
- `reinsert_split_targets`
- `on_catalytic_split_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/debug_sums.rs`

- `debug_check_sums`

### `src/graph/algorithm/plateau/dynamic_tracker/debug_diff.rs`

- `debug_assert_mirror_consistency_impl`

### `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/place.rs`

- `place_basis_element`
- `place_merge_both`
- `place_extend_left`
- `place_rekey_right`
- `place_new_plateau`

### `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs`

- `push_normalize_element`
- `process_normalize_node`
- `collect_from_basis_root`
- `collect_subtree_basis_elements`
- `place_sorted`
- `consolidate_basis_up`
- `consolidate_all_basis`

### `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs`

- `recompute_plateau`
- `fixup_plateau`
- `split_for_p_i4`
- `find_boundary_node`
- `evict_ancestor_key`
- `displace_semi_internal_survivor`
- `displace_path_siblings`
- `evacuate_adjacent_plateaus`

## Recommended Slice Order

1. **Phase 1**: Introduce `GNodeTree<C, V>` holding `nodes`, `root`,
   `node_count`, and `terminal_count`.  Provide `new`, `Default`,
   `From<Arena<GNode<C, V>>>`, `Deref`/`DerefMut` as transitional adapters.
   Switch `GTree.nodes` to `GNodeTree<C, V>`.  No method moves yet.
2. **Phase 2**: Move the cleanest structural methods onto `GNodeTree`:
   `uniform_contour_depth_of` (first, it is isolated), then `route_to`,
   `recompute_sums`, `recompute_sums_subtree`, and the allocation/link helpers.
   Keep `allocate_missing_child` on `GTree` for now because its counter
   accounting is deliberately separate.
3. **Phase 3**: Migrate plateau tracking from `&Arena<GNode<C, V>>` to
   `&GNodeTree<C, V>`.  Update `PlateauTracking` trait and both implementations.
4. **Phase 4**: Remove transitional `Deref`/`DerefMut`.  Replace all remaining
   implicit arena-style access with explicit `GNodeTree` API calls.
5. **Phase 5**: Final cleanup — rename `GTree.nodes` if agreed, run full gate,
   update this plan.

## Transitional Adapter Debt

All transitional adapters have been removed.  The table below shows their
full lifecycle.

| Item | Introduced | Removed | Notes |
|---|---|---|---|
| `Deref` / `DerefMut` from `GNodeTree` to `Arena<GNode<C, V>>` | Phase 1 | Phase 4 | Replaced by explicit delegation methods |
| `Deref` / `DerefMut` from `GTree` to `GNodeTree<C, V>` | Phase 1 | Phase 4 | Removed; callers updated to use `.nodes.*` paths |
| `GTree` field name `nodes` | pre-refactor | — | Kept; name is clear in context |

## Validation Gate

Primary gate for each migration slice:

```bash
cargo test --lib --tests
```

Recommended stricter gate when the slice touches plateau tracking heavily:

```bash
cargo test --lib --all-features
cargo test --test integration --all-features
cargo test --test snapshot_tests --all-features
bash scripts/clippy-strict.sh
```

## Done Criteria for This Refactor

All criteria have been met:

1. ✅ No production helper signatures expose `&Arena<GNode<C, V>>` for structural
   read access.
2. ✅ `GTree.nodes` stores `GNodeTree<C, V>`.
3. ✅ Structural G-node traversal helpers live on `GNodeTree`.
4. ✅ Plateau tracking reads G-node structure through `GNodeTree` or a higher
   owner, not the raw arena.
5. ✅ Transitional adapter debt table is empty.
6. ✅ Validation targets for all slices are green (529 tests pass).

## Remaining Work

None.  Refactor is complete.

## Out of Scope

- Replacing `Arena<T>` itself.
- Reworking plateau semantics.
- Changing G-tree budget or eviction policy.
- Folding `GTree` and `GNodeTree` into a single type.
