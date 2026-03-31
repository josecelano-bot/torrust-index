# Step 4 — `GvCore` cross-tree operation methods

## Goal

Move free functions whose bodies need **both** `self.gtree` **and** `self.vtree`
from the algorithm module into methods on `GvCore`, following the ownership-ladder
principle.

## Candidates

Only one free function in the codebase takes `&mut GvCore<C, V, N>` and is not
already a method:

| Free function | File | Notes |
|--|--|--|
| `resolve_path_b` | `algorithm/rebalance/resolve.rs` | Cross-tree check + calls `core.legacy_promote` |

The public entry point `resolve` also takes `&mut GvCore<C, V, N>`, but its body
is purely a dispatcher: it reads `core.vtree` directly and delegates all cross-tree
work to `resolve_path_b`.  Both are candidates; decision and scoping discussed below.

### Already on `GvCore` (context, not candidates)

| Method | Location | Note |
|--|--|--|
| `legacy_promote` | `core.rs` | cross-tree; migrated before this plan began |
| `rebalance` | `core.rs` | calls `rebalance::resolve(self, …)`; already a method |
| `alloc_v_entry` | `core.rs` | cross-tree helper; already a method |

## Why `resolve_path_b` belongs on `GvCore`

The function body:

1. Reads `core.vtree.nodes.structural_child_count(g)` and calls `core.vtree.contract(g)`.
2. Reads `core.vtree.nodes.get(c.index()).kind()` → `VKind::Entry { gnode, .. }` then
   checks `core.gtree.nodes.get(gnode.index()).is_semi_internal()` — a cross-tree check.
3. Calls `core.legacy_promote(c)` — an existing `GvCore` method.

Step 2 is the defining cross-tree access: `gnode_id` comes from the V-tree, but the
semi-internal flag is read from the G-tree.  No type lower than `GvCore` has both.

## Whether to move `resolve` as well

`resolve` also takes `&mut GvCore<C, V, N>`.  Its body accesses `core.vtree.nodes`
directly for lookups and delegates all structural mutations to VTree methods and to
`resolve_path_b`.  If `resolve_path_b` becomes a private `GvCore` method, the caller
inside `resolve` must change from
`resolve_path_b(core, c, p, g, depth_evict)` to `core.resolve_path_b(c, p, g, depth_evict)`.

At that point `resolve` is still a free function that takes `core: &mut GvCore`.
The question is whether moving `resolve` to `GvCore` as well is worth it.

**Arguments for moving `resolve` too:**
- `GvCore::rebalance` calls `rebalance::resolve(self, c, depth_evict)`; after the
  move this becomes `self.resolve_violation(c, depth_evict)` — consistent with how
  `rebalance` is already a method.
- Eliminates the last `pub` free function that takes `&mut GvCore`.

**Arguments against (obstacles):**
- `resolve` calls the escalate helper functions (`escalate_after_promote`,
  `resolve_try_contract_parent`) which are currently private to `resolve.rs` and
  only take `&mut VTree<V>`.  Moving `resolve` to `core.rs` requires those helpers
  to be accessible from `core.rs`, which means either:
  - Moving them to `core.rs` too (clutters a file whose purpose is GvCore data).
  - Promoting them to `pub(super)` in `resolve.rs` (violates "only exported symbols",
    since they are single-use helpers).
  - Moving them to `VTree` where they conceptually belong (this is a future step;
    doing it now exceeds the Step 4 scope).

**Decision:** Move only `resolve_path_b` in this step.  Update the single call
inside `resolve` from `resolve_path_b(core, …)` to `core.resolve_path_b(…)`.
Moving `resolve` itself is deferred to a future step once the escalate helpers have
been assessed for `VTree` migration.

## Migration pattern

```rust
// Before — free function in resolve.rs
fn resolve_path_b<C: Coordinate, V: Accumulator, const N: u32>(
    core: &mut GvCore<C, V, N>,
    c: VNodeId,
    p: VNodeId,
    g: VNodeId,
    depth_evict: u32,
) -> Option<GNodeId> { ... }

// After — private method on GvCore in core.rs
impl<C: Coordinate, V: Accumulator, const N: u32> GvCore<C, V, N> {
    fn resolve_path_b(
        &mut self,
        c: VNodeId,
        p: VNodeId,
        g: VNodeId,
        depth_evict: u32,
    ) -> Option<GNodeId> { ... }
}
```

The call site in `resolve` changes from:
```rust
let result = resolve_path_b(core, c, p, g, depth_evict);
```
to:
```rust
let result = core.resolve_path_b(c, p, g, depth_evict);
```

## Imports `core.rs` needs for the moved body

`resolve_path_b` body uses:

| Symbol | Where it comes from | Already in `core.rs`? |
|--|--|--|
| `VKind` | `crate::nodes::vnode` | no |
| `ViolationQueue` | `crate::graph::algorithm::violation_push` | no |
| `Ctx`, `Nd` | `crate::graph::algorithm::rebalance` (re-exports from `fmt`) | `Nd` is already used in `rebalance` method |

`Ctx` is already used in `core.rs` via `rebalance::Ctx`.  `VKind` and `ViolationQueue`
need to be added to the `use` block in `core.rs`.

## Files to change

| File | Action |
|--|--|
| `src/graph/core.rs` | Add `fn resolve_path_b` as a private method; add `use crate::nodes::vnode::VKind` and `use crate::graph::algorithm::violation_push::ViolationQueue` |
| `src/graph/algorithm/rebalance/resolve.rs` | Delete `fn resolve_path_b`; update the call site to `core.resolve_path_b(c, p, g, depth_evict)` |

## Tests

The three tests in `resolve.rs` (`resolve_returns_none_when_node_has_no_parent`,
`resolve_skip_path_returns_none_without_grandparent`,
`resolve_path_b_legacy_promote_returns_new_gnode`) test `resolve_path_b` and `resolve`
indirectly via the public `resolve` free function.  They do not need to move; the
public `resolve` entry point stays in `resolve.rs` and the tests remain valid.

## Commit message

```
refactor(gvcore): move resolve_path_b to GvCore method

resolve_path_b is the only remaining free function that performs a cross-tree
check (vtree entry → gtree semi-internal flag) and calls the existing
GvCore::legacy_promote method.  It is the natural owner.

Callers: one call site in resolve.rs updated to core.resolve_path_b(…).
```

## Verification

```bash
./scripts/verify.sh
```

All 598 tests (584 lib + 10 integration + 4 snapshot) must still pass.
Zero Clippy warnings. Zero spell-check issues.
