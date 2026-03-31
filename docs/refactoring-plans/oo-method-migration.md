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

### `is_violated` — threshold ownership problem (resolved)

`is_violated(vnodes, id)` was originally assumed to depend on a threshold from
`Config`, which would have prevented it from becoming a `VNodeTree` method.

In practice the function turned out to be a **purely structural comparison**:
it only compares uncle intensity between sibling V-nodes and does not consult
`alpha_relax` or any other config field. It migrated cleanly to
`VNodeTree::is_violated(&self, c: VNodeId) -> bool` with no config dependency.

## Suggested migration order

Work bottom-up. Each layer is prerequisite for the next.

1. **`VNodeTree` pure queries** — functions that only read `&VNodeTree`, no config
   dependency. Safest first step; zero callers need changing at the `VTree` level.
   Examples: `structural_child_count`, `v_depth_local`, `any_child_violated`.
   → Sub-plan: [step1-vnodetree-pure-queries.md](step1-vnodetree-pure-queries.md)

2. **`VTree` single-tree mutations** — functions that need `&mut VTree` (nodes +
   violations queue) and nothing from the G-tree or config.
   Examples: `contract`, `standard_promote`, `skip_promote`.
   (Escalate helpers stay in `resolve.rs` — they need algorithm-layer types.)
   → Sub-plan: [step2-vtree-mutations.md](step2-vtree-mutations.md)

3. **`GNodeTree` pure queries** — functions that only read `&GNodeTree`.
   → Sub-plan: [step3-gnodetree-queries.md](step3-gnodetree-queries.md)

4. **`GvCore` cross-tree operations** — functions that need both `self.gtree` and
   `self.vtree`. Examples: `legacy_promote` (already done), `resolve_path_b`.
   → Sub-plan: [step4-gvcore-cross-tree.md](step4-gvcore-cross-tree.md)

5. **`GvGraph`-level operations** — functions that additionally need `Config` or the
   plateau tracker.

### Checklist for each step (derived from Step 1 experience)

For every free function being migrated:

1. Add the method to the target type.
2. Remove the free function (or delete the whole source file if it becomes empty).
3. Remove any thin delegate that the algorithm module kept forwarding to the old
   free function — these are easy to miss and must be cleaned up in the same step.
4. Update **all** call sites across the codebase, not just the ones listed in the
   sub-plan — the real scope of callers is typically wider than anticipated.
5. Move the tests for the migrated function into the target type's module, even when
   the tests require a `GvGraph` fixture for setup.
6. Run `./scripts/verify.sh` to confirm zero warnings and all tests pass.

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

- [x] Step 1 — `VNodeTree` pure query methods
- [x] Step 2 — `VTree` mutation methods (`contract`, `standard_promote`, `skip_promote`)
- [x] Step 3 — `GNodeTree` pure query methods (`total_sum`, `gnode_children`, `is_ancestor_of`)
- [ ] Step 4 — `GvCore` cross-tree operation methods
- [ ] Step 5 — `GvGraph`-level operations

### Step 1 — implementation notes

**Methods added to `VNodeTree`** (all `pub(crate)`):
`max_uncle_intensity`, `is_violated`, `structural_child_count`, `any_child_violated`,
`find_violated_nodes`.

**`violation_scan.rs` deleted** — once `find_violated_nodes` moved, the file
contained nothing and was removed entirely.

**Caller scope wider than planned** — the sub-plan listed 5 files to update;
the actual set was 8:
`rebalance.rs`, `resolve.rs`, `fmt.rs`, `observe.rs`, `evict.rs`, `decay.rs`,
`core.rs`, `diagnostic.rs`, `invariants.rs`.

**Thin delegates were not removed in the initial commit** — `is_violated` and
`max_uncle_intensity` were left as forwarding stubs in `rebalance.rs` after the
methods were added. They had to be cleaned up in a separate follow-up commit
along with all the external caller updates. Future steps should make delegate
removal explicit in the commit plan.

**`is_violated` config dependency was a false alarm** — the function performed a
pure structural comparison with no reference to `alpha_relax` or any other config
field; it migrated to `VNodeTree` with no changes to its logic.

**Test co-location** — tests for the migrated methods were moved into
`vnode_tree.rs` alongside their implementations. Tests that require `GvGraph` as
a setup fixture can live there too; the module just imports `GvGraph` within the
`#[cfg(test)]` block.

### Step 2 — implementation notes

**Methods added to `VTree`** (all `pub(crate)`):
`contract`, `standard_promote`, `skip_promote`.

**`promote.rs` deleted** — all three functions migrated out; the file became empty
and was removed, and `pub mod promote` was deleted from `algorithm/mod.rs`.

**Caller scope matched the plan** — unlike Step 1, the caller scope was narrow and
well-predicted: all call sites were in `resolve.rs` and `split/helpers.rs`.

**Circular import prevented tracing span field re-use** — `Nd` and `Ch` display
wrappers live in `fmt.rs` which is not reachable from `vtree/mod.rs` without
a circular module dependency.  The tracing spans inside `contract`,
`standard_promote`, and `skip_promote` were simplified to emit raw `.index()`
integers rather than the formatted wrappers.

**Unused re-export clean-up** — `rebalance.rs` had `pub(super) use super::fmt::Ch`
for use by the deleted `contract` body.  Once `contract` was removed the re-export
became dead.  Clippy caught this and it was removed; the companion test
`ch_display_fn` was updated to import `Ch` directly from `fmt`.

### Step 3 — implementation notes

**Pattern differed from Steps 1 and 2** — no free functions existed to migrate;
the candidates were `GvGraph` methods whose bodies only read `GNodeTree` fields.
Step 3 is *method demotion*: push the implementation down to the lowest type that
has sufficient data.

**Methods added to `GNodeTree`** (all `pub(crate)`):
`total_sum`, `gnode_children`, `is_ancestor_of`.

**No files deleted** — unlike Steps 1 and 2, no source file became empty.
`GvGraph` kept thin one-line delegates; its public API is unchanged.

**`GNodeChildren` import added** — `gnode_tree.rs` did not previously import
`GNodeChildren`; one `use` line was added to support the new `gnode_children` method.

**Caller scope zero** — all callers continue going through the `GvGraph` delegates;
no call site outside `gv_graph.rs` needed updating.

**No test migration** — existing `GvGraph` tests exercise the three methods through
the public API and remain valid without modification.
