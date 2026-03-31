# Step 1 — `VNodeTree` Pure Query Methods

Sub-plan of [oo-method-migration.md](oo-method-migration.md).

## Goal

Move all free functions that only need `&VNodeTree<V>` into methods on `VNodeTree`.
No algorithmic changes; no `&mut` access; no `Config`, `GTree`, or `VTree` dependency.

## Candidates

All candidates are in `src/graph/algorithm/rebalance/` and its sub-modules.

### A. Already a `VNodeTree` method — delete the free function, update callers

These free functions duplicate something `VNodeTree` already provides.

| Free function | Location | `VNodeTree` method | Action |
|---|---|---|---|
| `node_has_evictable(vnodes, id)` | `rebalance.rs:113` — `pub(super)` | `vnodes.node_has_evictable(id)` already exists | Delete free fn; update callers in `rebalance.rs` |
| `v_depth_local(vnodes, id)` | `resolve.rs:30` | `vnodes.depth(id)` already exists | Delete free fn; inline `vnodes.depth(id)` at callsite |
| `v_depth_local(vnodes, id)` | `violation_scan.rs:7` | `vnodes.depth(id)` already exists | Delete free fn; inline `vnodes.depth(id)` at callsite |

### B. New `VNodeTree` methods — add and migrate

These functions have no equivalent on `VNodeTree` yet.

| Free function | Location | Proposed method signature | Notes |
|---|---|---|---|
| `max_uncle_intensity(vnodes, c)` | `rebalance.rs:16` — `pub` | `pub(crate) fn max_uncle_intensity(&self, c: VNodeId) -> Option<V>` | Pure traversal; no threshold |
| `is_violated(vnodes, c)` | `rebalance.rs:40` — `pub` | `pub(crate) fn is_violated(&self, c: VNodeId) -> bool` | Calls `max_uncle_intensity`; no threshold — pure read |
| `structural_child_count(vnodes, id)` | `resolve.rs:11` — private | `pub(crate) fn structural_child_count(&self, id: VNodeId) -> usize` | One-liner; delta = ergonomics at callsite |
| `any_child_violated(vnodes, node)` | `resolve.rs:15` — private | `pub(crate) fn any_child_violated(&self, node: VNodeId) -> bool` | Calls `is_violated`; becomes `self.is_violated(child_id)` once B is done |
| `find_violated_nodes(vnodes)` | `violation_scan.rs:12` — `pub` | `pub(crate) fn find_violated_nodes(&self) -> Vec<VNodeId>` | Full scan; needs `v_depth_local` deleted first (group A) |

## Ordering within this step

1. Group A first (deleting duplicates is zero-risk).
2. `max_uncle_intensity` → `is_violated` (dependency order).
3. `structural_child_count` (trivial).
4. `any_child_violated` (depends on `is_violated` being a method).
5. `find_violated_nodes` (depends on `v_depth_local` being deleted — replaced by `self.depth(id)`).

## Callers to update after migration

`is_violated` and `max_uncle_intensity` are called throughout the rebalance module.
Expect changes in:

- `src/graph/algorithm/rebalance.rs` — `is_violated`, `max_uncle_intensity` callsites
- `src/graph/algorithm/rebalance/resolve.rs` — `is_violated`, `structural_child_count`, `any_child_violated`
- `src/graph/algorithm/rebalance/violation_scan.rs` — after `find_violated_nodes` moves, this file may become very thin or disappear entirely
- `src/diagnostics/diagnostic.rs` — calls `rebalance::is_violated` (indirectly via `find_violated_nodes`)
- `src/graph/core.rs` — calls `rebalance::is_violated`

## Scope boundaries

- Only `src/tree/vtree/vnode_tree.rs` gains new methods.
- No changes to `VTree`, `GTree`, `GvCore`, or `GvGraph`.
- No changes to algorithm logic — only signature / location changes.
- `pub(crate)` visibility on all new methods (they are not part of the public crate API).
- Run `./scripts/verify.sh` after the full group is done.

## Commit plan

```
refactor(vnodetree): delete duplicate free fns node_has_evictable, v_depth_local
refactor(vnodetree): add max_uncle_intensity and is_violated methods
refactor(vnodetree): add structural_child_count and any_child_violated methods
refactor(vnodetree): move find_violated_nodes from violation_scan to VNodeTree
```

## Progress

- [ ] Group A — delete `node_has_evictable` free fn, both `v_depth_local` free fns
- [ ] `max_uncle_intensity` → `VNodeTree` method
- [ ] `is_violated` → `VNodeTree` method
- [ ] `structural_child_count` → `VNodeTree` method
- [ ] `any_child_violated` → `VNodeTree` method
- [ ] `find_violated_nodes` → `VNodeTree` method
