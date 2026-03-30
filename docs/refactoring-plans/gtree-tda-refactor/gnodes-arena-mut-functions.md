# Functions Taking `&mut Arena<GNode...>`

Inventory of functions found with a mutable G-node arena parameter.

Status reflects the completed GTree Tell-Don't-Ask refactor.

## Production Code

| File | Function | Old Shape | Resolution |
|---|---|---|---|
| `src/graph/algorithm/promote.rs` | `legacy_promote` | `gnodes: &mut Arena<GNode<C, V>>` | Now takes `gtree: &mut GTree<C, V, N>` |
| `src/graph/algorithm/rebalance.rs` | `rebalance` | `gnodes: &mut Arena<GNode<C, V>>` | Now takes `gtree: &mut GTree<C, V, N>` |
| `src/graph/algorithm/rebalance/resolve.rs` | `resolve_path_b` | `gnodes: &mut Arena<GNode<C, V>>` | Now takes `gtree: &mut GTree<C, V, N>` |
| `src/graph/algorithm/rebalance/resolve.rs` | `resolve` | `gnodes: &mut Arena<GNode<C, V>>` | Now takes `gtree: &mut GTree<C, V, N>` |
| `src/graph/algorithm/split/helpers.rs` | `alloc_v_entry` | `gnodes: &mut Arena<GNode<C, V>>` | Now takes `gtree: &mut GTree<C, V, N>` |
| `src/tree/vtree.rs` | `VTree::remove_leaf` | `gnodes: &mut Arena<GNode<C, V>>` | Now takes `gtree: &mut GTree<C, V, N>` |

## Test Scaffolding Still Using Raw Arenas

These are local test helpers, not production APIs.

| File | Function | Notes |
|---|---|---|
| `src/graph/algorithm/plateau/mod.rs` | `add_leaf` | Test helper for plateau wiring |
| `src/graph/algorithm/plateau/mod.rs` | `add_node` | Test helper for plateau wiring |
| `src/graph/algorithm/plateau/dynamic_tracker/tests.rs` | `add_leaf` | Test helper |
| `src/graph/algorithm/plateau/dynamic_tracker/tests.rs` | `add_node` | Test helper |
| `src/graph/algorithm/plateau/dynamic_tracker/coverage_tests.rs` | `add_leaf` | Test helper |
| `src/graph/algorithm/plateau/dynamic_tracker/coverage_tests.rs` | `add_node` | Test helper |

## New `GTree` Mutator Surface Added

These methods were added so cross-tree algorithms no longer need direct mutable arena access:

| File | Method | Purpose |
|---|---|---|
| `src/tree/gtree.rs` | `allocate_missing_child` | Allocate and link the missing child of a semi-internal G-node |
| `src/tree/gtree.rs` | `assign_entry` | Route `GNode::assign_entry` through `GTree` |
| `src/tree/gtree.rs` | `clear_entry` | Route `GNode::clear_entry` through `GTree` |
