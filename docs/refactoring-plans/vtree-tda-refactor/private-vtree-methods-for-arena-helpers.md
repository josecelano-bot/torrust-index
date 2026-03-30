# Refactor Plan: Convert `vnodes: &mut Arena<VNode<V>>` Helpers into Private `VTree` Methods

## Motivation

`src/tree/vtree.rs` still has several free helper functions that operate on
`vnodes: &mut Arena<VNode<V>>`. These helpers represent `VTree` behavior and are
already invoked mainly from `VTree` methods. Converting them into private
methods improves encapsulation and removes repetitive `&mut self.nodes`
threading.

## Scope

Target only helper functions in `src/tree/vtree.rs` that currently take
`vnodes: &mut Arena<VNode<V>>`:

- `propagate_v_sums`
- `recompute_all_v_intensities`
- `recompute_v_postorder`
- `propagate_evictable_flags`
- `sync_intensity_in_parent`
- `recompute_structural_intensity`
- `recompute_and_sync_parent_slot`
- `recompute_and_propagate_v_sums`

Out of scope:

- Cross-module API changes in `src/graph/algorithm/*`
- Behavior changes to balancing/eviction logic
- Structural utility `sole_sibling` (does not take `&mut Arena`)

## Steps

1. Add private methods in `impl<V: Accumulator> VTree<V>` for each target helper.
2. Replace internal call sites to use `self.<method>(...)`.
3. Remove free helper function definitions that were converted.
4. Update comments/doc links to refer to method names where needed.
5. Update unit tests that directly call removed free helpers.
6. Run tests for `vtree` and fix any borrow checker regressions introduced by method conversion.

## Acceptance Criteria

- No free functions remain in `src/tree/vtree.rs` that take
  `vnodes: &mut Arena<VNode<V>>`.
- `VTree` compiles and tests pass for affected module behavior.
- Public behavior is unchanged:
  - leaf removal paths (`root`, `shrink`, `collapse`) still pass
  - sum propagation/recompute semantics remain unchanged
  - evictable propagation remains unchanged

## Risks and Mitigations

- Risk: Method conversion can trigger borrow checker conflicts in recursive or
  chained mutation paths.
- Mitigation: Keep borrows tightly scoped (collect IDs first, then mutate);
  avoid holding references across method calls.
