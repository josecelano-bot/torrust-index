# OO Method Migration

## Vision

The codebase currently uses a heavily imperative style: many algorithms are expressed
as free functions that receive the data they need as explicit parameters. The goal of
this refactoring is to move those free functions onto the types that own the minimum
data required, making the code more object-oriented in the sense of behaviour living
alongside data.

The guiding rule is:

> A free function that takes `&T` or `&mut T` as its first (or only) meaningful
> argument is a candidate to become a method on `T`.

## Ownership ladder

Types form a natural hierarchy based on what they own:

```
GvGraph          — GvCore + Config + PlateauTracker
  GvCore         — GTree + VTree  (dual-tree cross-cutting operations)
    GTree        — GNodeTree + depth policy parameters
      GNodeTree  — Arena<GNode> + root + counters
    VTree        — VNodeTree + violations queue
      VNodeTree  — Arena<VNode> + root
```

A free function should live as a method on the *lowest* type in this ladder whose
data is sufficient for the function's implementation.

## Motivation

1. **Discoverability** — callers find behaviour by navigating the type's API, not by
   knowing which free-function module to import.
2. **Encapsulation** — `pub(crate)` on a method is cleaner than `pub(crate)` on a
   free function scattered across algorithm submodules.
3. **Borrow ergonomics** — methods express the borrow at the call site instead of
   threading `&mut self.vtree.nodes` through multiple argument lists.
4. **Cohesion** — the data and the behaviour that acts on it sit in the same module.

## Algorithms are distributed across layers

An algorithm does not live in one place. Each type contributes the part of the
algorithm that only it has the data to perform. A thin coordinator (the algorithm
module) orchestrates across types.

```
Algorithm "promote"
  VNodeTree layer  — structural queries needed by all promote variants
                     (child count, sibling lookup, evictable flag reads)
  VTree layer      — structural mutations + propagation
                     (standard_promote, skip_promote — pure V-tree operations)
  GvCore layer     — cross-tree mutations
                     (legacy_promote — touches both GTree and VTree)
  algorithm module — entry point / dispatch
                     (resolve, which selects among the promote variants)
```

The algorithm module becomes a coordinator that calls methods on the appropriate
layer, not a bag of free functions that each replicate the parameter threading.

### Promote algorithm — concrete layering example

**`standard_promote`** only needs `&mut VTree`, so it belongs on `VTree`:

```rust
// Before — free function in src/graph/algorithm/promote.rs
pub fn standard_promote<V: Accumulator>(vtree: &mut VTree<V>, c: VNodeId) { ... }

// After — method on VTree
impl<V: Accumulator> VTree<V> {
    pub(crate) fn standard_promote(&mut self, c: VNodeId) { ... }
}
```

The algorithm module entry point (`resolve`) then calls `core.vtree.standard_promote(c)`
instead of `standard_promote(&mut core.vtree, c)` — a purely ergonomic improvement.

**`skip_promote`** is identical: only needs `&mut VTree` → method on `VTree`.

**`legacy_promote`** needs both `self.gtree` and `self.vtree` → stays on `GvCore`
(or gets inlined into `resolve_path_b` once `GvCore` is accessible there directly).

### `VNodeTree` pure query candidates (in `resolve.rs`)

```rust
// Before
fn structural_child_count<V: Accumulator>(vnodes: &VNodeTree<V>, id: VNodeId) -> usize {
    vnodes.get(id.index()).child_count()
}

fn any_child_violated<V: Accumulator>(vnodes: &VNodeTree<V>, node: VNodeId) -> bool { ... }

fn v_depth_local<V: Accumulator>(vnodes: &VNodeTree<V>, id: VNodeId) -> u32 {
    vnodes.depth(id)
}

// After — methods on VNodeTree
impl<V: Accumulator> VNodeTree<V> {
    pub(crate) fn child_count(&self, id: VNodeId) -> usize { ... }
    pub(crate) fn any_child_violated(&self, node: VNodeId, is_violated: impl Fn(VNodeId) -> bool) -> bool { ... }
    // depth() already exists
}
```

### `is_violated` — threshold ownership problem

`is_violated(vnodes, id)` currently threads the violation threshold implicitly through
its callers. The threshold is owned by `Config` (on `GvGraph`), not by `VNodeTree`.
This is why it lives as a free function rather than a method today.

Two resolution paths exist:

- Move `alpha_relax` into `VTree` so that `VTree::is_violated(id)` can close over it.
- Keep `is_violated` as a free function but give it a single well-typed home
  (e.g. `rebalance::is_violated`) and call it consistently.

## Suggested migration order

Work bottom-up. Each layer is prerequisite for the next.

1. **`VNodeTree` pure queries** — functions that only read `&VNodeTree`, no config
   dependency. Safest first step; zero callers need changing at the `VTree` level.
   Examples: `structural_child_count`, `v_depth_local`, `any_child_violated`.

2. **`VTree` single-tree mutations** — functions that need `&mut VTree` (nodes +
   violations queue) and nothing from the G-tree or config.
   Examples: `standard_promote`, `skip_promote`, `escalate_contract_parent`,
   `escalate_try_contract_grandparent`, `escalate_skip_promote`,
   `escalate_after_promote`, `resolve_try_contract_parent`.

3. **`GNodeTree` pure queries** — functions that only read `&GNodeTree`.

4. **`GvCore` cross-tree operations** — functions that need both `self.gtree` and
   `self.vtree`. Examples: `legacy_promote` (already done), `resolve_path_b`.

5. **`GvGraph`-level operations** — functions that additionally need `Config` or the
   plateau tracker.

## Scope boundaries

- This refactoring is **purely mechanical**: no algorithmic changes, no behaviour
  changes.
- Each step should be a separate commit, verified clean by `./scripts/verify.sh`.
- Free functions that are genuinely stateless utilities (e.g. `gnode_depth_from_interval`)
  may stay as free functions if they do not belong to any single type.
- Algorithm entry points (`resolve`, `rebalance`) that are called from multiple places
  may stay as `pub(crate)` free functions or become methods on `GvCore`; decide
  case-by-case based on the call-site ergonomics gained.

## Progress

- [ ] Step 1 — `VNodeTree` pure query methods
- [ ] Step 2 — `VTree` operation methods
- [ ] Step 3 — `GNodeTree` pure query methods
- [ ] Step 4 — `GvCore` cross-tree operation methods
- [ ] Step 5 — `GvGraph`-level operations
