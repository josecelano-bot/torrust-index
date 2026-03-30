# Functions Taking `vnodes: &mut Arena<VNode<V>>` — Refactoring Inventory

All functions that originally accepted `vnodes: &mut Arena<VNode<V>>` as a
parameter. Status reflects the completed TDA refactor.

## `src/graph/algorithm/promote.rs` — ✅ Resolved

| Function | Old Visibility | Resolution |
|---|---|---|
| `standard_promote` | `pub` | Now takes `&mut VTree<V>` |
| `skip_promote` | `pub` | Now takes `&mut VTree<V>` |
| `legacy_promote` | `pub` | Now takes `&mut VTree<V>` |

## `src/graph/algorithm/rebalance.rs` — ✅ Resolved

| Function | Old Visibility | Resolution |
|---|---|---|
| `contract` | `pub` | Now takes `&mut VTree<V>` |

## `src/graph/algorithm/split/helpers.rs` — ✅ Resolved

| Function | Old Visibility | Resolution |
|---|---|---|
| `alloc_v_entry` | `pub(super)` | Now takes `&mut VTree<V>` |
| `alloc_v_structural_2` | `pub(super)` | Moved to `VTree::alloc_structural_2` |

## `src/tree/vtree.rs` — ✅ Resolved

| Function | Old Visibility | Resolution |
|---|---|---|
| `propagate_v_sums` | `fn` (private) | Unchanged — internal V-tree arithmetic |
| `recompute_all_v_intensities` | `fn` (private) | Unchanged — internal V-tree arithmetic |
| `recompute_v_postorder` | `fn` (private) | Unchanged — internal V-tree arithmetic |
| `propagate_evictable_flags` | `pub` | Demoted to `fn` (private) |
| `sync_intensity_in_parent` | `fn` (private) | Unchanged — internal V-tree arithmetic |
| `recompute_structural_intensity` | `fn` (private) | Unchanged — internal V-tree arithmetic |
| `recompute_and_sync_parent_slot` | `pub` | Demoted to `fn` (private) |
| `recompute_and_propagate_v_sums` | `fn` (private) | Unchanged — internal V-tree arithmetic |

## `src/tree/vtree/mutation.rs` — ✅ Deleted

| Function | Old Visibility | Resolution |
|---|---|---|
| `replace_child_in_parent` | `pub` | Inlined into `VTree::replace_structural_child` |
| `add_child_to_structural` | `pub` | Inlined into `VTree::add_structural_child` |
| `set_entry_flags` | `pub` | Inlined into `VTree::set_entry_flags` |
| `remove_child_from_structural` | `pub` | Inlined into `VTree::remove_structural_child` |
| `set_has_evictable` | `pub` | Inlined into `propagate_evictable_flags` (private) |
