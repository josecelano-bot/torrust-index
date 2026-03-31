# Step 3 — `GNodeTree` pure query methods

## Goal

Move `GvGraph` methods whose bodies only read `GNodeTree` fields down to
`GNodeTree` itself, following the ownership-ladder principle: behaviour lives on
the *lowest* type that has sufficient data.

## Key difference from Steps 1 and 2

Steps 1 and 2 migrated *free functions* that lived in separate algorithm modules
(`violation_scan.rs`, `promote.rs`).  Step 3 has **no such free functions** —
the candidates are already `impl GvGraph` methods.  The migration pattern here
is *method demotion*: a method that belongs at the `GNodeTree` layer is promoted
up to `GvGraph` and must be pushed back down.

## Candidates

### Eligible — body only touches `GNodeTree`

| `GvGraph` method | Access pattern | New home |
|--|--|--|
| `gnode_children` | `nodes.is_occupied`, `g.left()`, `g.right()` | `GNodeTree::gnode_children` |
| `is_ancestor_of` | `nodes.is_occupied`, `g.lo()`, `g.hi()` | `GNodeTree::is_ancestor_of` |
| `total_sum` | `nodes.get(root.index()).sum()` | `GNodeTree::total_sum` |

### Ineligible — require `const N` from `GTree`

| `GvGraph` method | Blocker |
|--|--|
| `gnode_depth` | calls `GTree::<C,V,N>::depth_of_interval` |
| `gnode_info` | calls `GTree::<C,V,N>::depth_of_interval` |

These methods build a `depth` field by calling a `const N`-generic static on
`GTree`.  `GNodeTree` is not parameterised over `N`, so these methods cannot
descend until `depth` is removed from the return type or computed elsewhere.

### Excluded by design — plain field delegates

`node_count()`, `terminal_count()`, and `g_root()` on `GvGraph` are single-line
field reads that already delegate through `self.core.gtree.nodes.*`.  They do not
introduce any real algorithmic coupling at the `GvGraph` level, so pushing them
to `GNodeTree` would only add a forwarding chain with no ergonomic gain.

## Visibility rules

`GNodeTree` lives inside `src/tree/gtree/gnode_tree.rs` and is `pub(crate)` to
the outside world but publicly accessible to the whole crate.  All three new
methods should be `pub(crate)` — the same visibility the `GvGraph` wrappers
currently have (`pub`, `pub`, `pub` respectively — so the wrappers must stay on
`GvGraph` as thin delegates that call down to `GNodeTree`).

For `gnode_children` and `is_ancestor_of`, which are `pub` on `GvGraph`, the
`GvGraph` method must remain and simply delegate to the new `GNodeTree` method.
For `total_sum`, which is also `pub`, same pattern applies.

## Migration pattern for each method

```
// Step A — add to GNodeTree (gnode_tree.rs)
pub(crate) fn gnode_children(&self, id: GNodeId) -> Option<GNodeChildren> { ... }

// Step B — update GvGraph body to delegate
pub fn gnode_children(&self, id: GNodeId) -> Option<GNodeChildren> {
    self.core.gtree.nodes.gnode_children(id)
}
```

## Files to change

| File | Action |
|--|--|
| `src/tree/gtree/gnode_tree.rs` | Add `gnode_children`, `is_ancestor_of`, `total_sum` |
| `src/graph/gv_graph.rs` | Replace method bodies with one-line delegates |

No other call sites need changing: callers always go through `GvGraph`, which
stays in place.  The `GvGraph` tests that exercise these methods remain valid
without modification.

## Commit plan

One commit for the implementation:

```
refactor(gnodetree): move gnode_children, is_ancestor_of, total_sum to GNodeTree

Methods whose bodies only read GNodeTree fields are demoted from GvGraph to
GNodeTree.  GvGraph keeps thin delegates so the public API is unchanged.
```

## `GNodeTree` module placement

Add the three new methods in a new `// ── Pure queries ─────────────────────────────────────────────────────────` section
inside the `impl<C: Coordinate, V: Accumulator> GNodeTree<C, V>` block in
`gnode_tree.rs`, after the existing `// ── Depth helpers ────────────────────────────────────────────────────────`
section and before the allocation helpers.

## Verification

```bash
./scripts/verify.sh
```

All 598 tests (584 lib + 10 integration + 4 snapshot) must still pass.
Zero Clippy warnings.
