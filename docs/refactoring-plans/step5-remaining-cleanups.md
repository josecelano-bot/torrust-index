# Step 5 — Remaining free-function clean-up

## Scope correction

The original Step 5 entry said *"functions that additionally need `Config` or the
plateau tracker"*.  The survey found **no such free functions** in the codebase:
every algorithm module operates through `GvGraph` methods already, and no
free function at module scope touches `Config`, `PlateauTracking`, or
`DynamicPlateauTracker`.

Step 5 is therefore re-scoped as a **deferred clean-up pass** covering free
functions at lower layers that were out of scope for Steps 1–4 (because they were
private helpers that sit inside a module together with the functions that call them)
plus the `resolve` migration that was explicitly deferred from Step 4.

---

## Candidate inventory

There are four distinct groups.

### Group A — `resolve` → `GvCore` (deferred from Step 4)

| Free function | File | Layer |
|--|--|--|
| `pub fn resolve` | `algorithm/rebalance/resolve.rs` | GvCore |

`resolve` takes `core: &mut GvCore<C, V, N>` and calls `core.resolve_path_b` (now
a method), `core.vtree.*`, and the private escalate helpers — all of which either
already belong to `GvCore` or belong to `VTree` (Group B below).

**Blocker from Step 4**: the escalate helpers (`escalate_after_promote`,
`escalate_contract_parent`, `escalate_try_contract_grandparent`,
`escalate_skip_promote`, `resolve_try_contract_parent`) are currently private free
functions in `resolve.rs` that only take `&mut VTree<V>`.  Moving `resolve` to
`core.rs` requires those helpers to be reachable from `core.rs`.  Options:
(a) migrate them to `VTree` first (Group B), then move `resolve`; or
(b) keep them as private free functions but declare them `pub(super)` and move the
module boundary.

**Decision**: do Group B first; `resolve` migration follows as a second commit.

After the migration, `GvCore::rebalance` changes from:
```rust
let gid = rebalance::resolve(self, c, depth_evict);
```
to:
```rust
let gid = self.resolve_violation(c, depth_evict);
```

### Group B — Escalate helpers → `VTree` methods (prerequisite for Group A)

| Free function | File | New home |
|--|--|--|
| `escalate_contract_parent` | `resolve.rs` | `VTree::escalate_contract_parent` |
| `escalate_try_contract_grandparent` | `resolve.rs` | `VTree::escalate_try_contract_grandparent` |
| `escalate_skip_promote` | `resolve.rs` | `VTree::escalate_skip_promote` |
| `escalate_after_promote` | `resolve.rs` | `VTree::escalate_after_promote` |
| `resolve_try_contract_parent` | `resolve.rs` | `VTree::resolve_try_contract_parent` |

All five take only `&mut VTree<V>` (plus lightweight value types), making them
correct `VTree` methods.

**Visibility**: these are implementation details of the `resolve` algorithm and are
only called within `resolve.rs`.  After migrating to `vtree/mod.rs` they should be
`pub(crate)` so `resolve.rs` (in a different module) can call them.

**`EscalationContext`** is also defined in `resolve.rs`.  It is a plain data struct
used only by the escalate helpers.  Move it to `vtree/mod.rs` alongside the methods,
or to a new `src/tree/vtree/escalation.rs` sub-module.  Since it is already
re-exported via `context.rs` (which re-exports it publicly as
`pub use context::EscalationContext`), one clean option is:
- Keep `EscalationContext` in `context.rs` (already there).
- The new `VTree` methods accept it as a parameter — the type lives in the
  `rebalance` module which `vtree` can import via `crate::graph::algorithm::rebalance`.

**Circular-import risk**: `vtree/mod.rs` would need to import
`crate::graph::algorithm::rebalance::EscalationContext`.  The `rebalance` module
currently does not import anything from `vtree/mod.rs`, so no cycle is introduced.
Verify with `cargo check` immediately after the first edit.

### Group C — `classify_leaf_removal` + `push_eviction_violations` → `VNodeTree` (deferred from Step 1)

| Free function | File | New home | Notes |
|--|--|--|--|
| `classify_leaf_removal` | `algorithm/evict.rs` | `VNodeTree::classify_leaf_removal` | also needs `LeafRemovalContext` |
| `push_eviction_violations` | `algorithm/evict.rs` | `VNodeTree::push_eviction_violations` | takes `&VNodeTree`, `v_id`, `ctx`, `queue` |

Both are private helpers called only from `GvGraph::evict_tip` in the same file.
`LeafRemovalContext` is a private struct in `evict.rs`; it must move to `vnode_tree.rs`
alongside its methods.  `ViolationQueue` is a parameter — `vnode_tree.rs` can import
it from `algorithm::violation_push`.

**Circular-import risk**: `vnode_tree.rs` importing from `algorithm::violation_push`
— `violation_push.rs` already imports `vnode_tree::VNodeTree`, so this would create a
cycle.  Resolution options:
- Keep `classify_leaf_removal` and `push_eviction_violations` as private free
  functions in `evict.rs` and mark them with a `// intentionally private to evict.rs`
  comment — they are single-caller helpers.
- Move `ViolationQueue` to a lower module (e.g. `tree/vtree/`) to break the cycle.

**Decision**: investigate the cycle at implementation time.  If the cycle is
unavoidable without a larger restructuring, leave these two functions in `evict.rs`
with an explanatory comment; they are private and do not pollute the public API.

### Group D — `violation_push.rs` free functions — intentionally stay free

The sixteen public free functions in `violation_push.rs` are a cohesive, named
family of single-purpose push operations.  They are already well-documented, all
take `&VNodeTree` (not `evict.rs` or GvCore data), and are consumed via the
`ViolationQueue` wrapper.  Converting them to `VNodeTree` methods would pollute
`VNodeTree`'s API with sixteen concern-specific methods.

**Decision**: these remain free functions.  They are in-scope for assessment but
are excluded from migration.

---

## Migration order

| Sub-step | Work | Precondition | Status |
|--|--|--|--|
| 5-A | Move escalate helpers to `VTree` | none | ⛔ blocked — see below |
| 5-B | Move `resolve` to `GvCore::resolve_violation` | 5-A was not required | ✅ done |
| 5-C | Attempt `classify_leaf_removal` + `push_eviction_violations` migration | none | ⛔ blocked — see below |
| 5-D | Update `oo-method-migration.md` with implementation notes | after each commit | ✅ done |

5-A and 5-C can be worked in any order; 5-B must follow 5-A.

## Blocked sub-steps — circular import analysis

### 5-A (escalate helpers → VTree)

The escalate helpers call `ViolationQueue::new(&mut vtree.violations)` whose type
lives in `violation_push.rs`.  `violation_push.rs` in turn imports
`crate::tree::vtree::VNodeTree`.  Adding an import of `ViolationQueue` from
`vtree/mod.rs` would therefore create the cycle:

```
crate::tree::vtree  →  crate::graph::algorithm::violation_push  →  crate::tree::vtree
```

The helpers are left in `resolve.rs` as private free functions.  They are
called only from `resolve` (now via `GvCore::resolve_violation`) and do not
appear in any public API.

### 5-C (evict helpers → VNodeTree)

Same root cause: `push_eviction_violations` takes `&mut ViolationQueue<'_>`.
Moving it to `VNodeTree` creates the identical cycle through `violation_push.rs`.

`classify_leaf_removal` alone has no cycle (it only reads `VNodeTree`), but
separating it from its only consumer `push_eviction_violations` would scatter
related logic with no ergonomic gain.  Both functions stay in `evict.rs` as
private helpers.

To unblock these sub-steps a future refactor would need to extract
`ViolationQueue` (or the raw `push_*` call + `&mut Vec<VNodeId>` pattern) into
a module that neither `vtree` nor `violation_push` depends on.

---

## Commit plan

```
5-A  refactor(vtree): move escalate helpers to VTree methods

     escalate_contract_parent, escalate_try_contract_grandparent,
     escalate_skip_promote, escalate_after_promote,
     resolve_try_contract_parent moved from resolve.rs to vtree/mod.rs.
     All pub(crate); EscalationContext stays in rebalance/context.rs.

5-B  refactor(gvcore): move resolve to GvCore::resolve_violation

     pub fn resolve free function replaced by
     pub(crate) fn resolve_violation on GvCore.
     GvCore::rebalance call site updated.
     resolve.rs remains for EscalationContext, Ctx/Nd re-exports and tests.

5-C  refactor(vnodetree): move classify_leaf_removal + push_eviction_violations
     (commit only if circular import is avoided)
```

---

## Files to change per sub-step

### 5-A

| File | Action |
|--|--|
| `src/tree/vtree/mod.rs` | Add five new `pub(crate)` methods; add `use crate::graph::algorithm::rebalance::EscalationContext` |
| `src/graph/algorithm/rebalance/resolve.rs` | Delete five free functions; update five call sites to `vtree.escalate_*` / `vtree.resolve_try_contract_parent` |

### 5-B

| File | Action |
|--|--|
| `src/graph/core.rs` | Add `pub(crate) fn resolve_violation` method; update `rebalance` loop call site |
| `src/graph/algorithm/rebalance/resolve.rs` | Delete `pub fn resolve`; optionally remove `use crate::graph::core::GvCore` if no longer needed |
| `src/graph/algorithm/rebalance/resolve.rs` tests | Update `resolve(&mut g.core, …)` → `g.core.resolve_violation(…)` |

### 5-C (conditional)

| File | Action |
|--|--|
| `src/tree/vtree/vnode_tree.rs` | Add `classify_leaf_removal`, `push_eviction_violations`; add `LeafRemovalContext` struct |
| `src/graph/algorithm/evict.rs` | Delete private functions + struct; update call sites |

---

## Verification

```bash
./scripts/verify.sh
```

All 598 tests (584 lib + 10 integration + 4 snapshot) must pass after each sub-step.
Zero Clippy warnings. Zero spell-check issues.
