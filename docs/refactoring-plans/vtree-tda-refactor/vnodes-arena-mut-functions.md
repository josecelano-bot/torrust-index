# Functions Taking `vnodes: &mut Arena<VNode<V>>`

List of all functions that accept `vnodes: &mut Arena<VNode<V>>` as a parameter.

## `src/graph/algorithm/promote.rs`

| Function | Visibility |
|---|---|
| `standard_promote` | `pub` |
| `skip_promote` | `pub` |
| `legacy_promote` | `pub` |

## `src/graph/algorithm/rebalance.rs`

| Function | Visibility |
|---|---|
| `contract` | `pub` |

## `src/graph/algorithm/split/helpers.rs`

| Function | Visibility |
|---|---|
| `alloc_v_entry` | `pub(super)` |
| `alloc_v_structural_2` | `pub(super)` |

## `src/tree/vtree.rs`

| Function | Visibility |
|---|---|
| `propagate_v_sums` | `fn` (private) |
| `recompute_all_v_intensities` | `fn` (private) |
| `recompute_v_postorder` | `fn` (private) |
| `propagate_evictable_flags` | `pub` |
| `sync_intensity_in_parent` | `fn` (private) |
| `recompute_structural_intensity` | `fn` (private) |
| `recompute_and_sync_parent_slot` | `pub` |
| `recompute_and_propagate_v_sums` | `fn` (private) |

## `src/tree/vtree/mutation.rs`

| Function | Visibility |
|---|---|
| `replace_child_in_parent` | `pub` |
| `add_child_to_structural` | `pub` |
| `set_entry_flags` | `pub` |
| `remove_child_from_structural` | `pub` |
| `set_has_evictable` | `pub` |
