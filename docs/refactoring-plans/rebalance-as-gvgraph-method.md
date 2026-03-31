# Refactoring Plan: `rebalance` as a `GvGraph` method

**Date:** March 31, 2026
**Component:** `src/graph/algorithm/rebalance.rs`, `observe.rs`, `decay.rs`, `budget.rs`
**Priority:** Low (code quality)
**Status:** Completed

---

## Executive Summary

The free function `rebalance::rebalance(&mut vtree, &mut gtree, depth_evict)` is called in
exactly three production places, and all three share an identical two-line pattern:

```rust
let depth_evict = self.gtree.live_depth_evict;
let new_gnodes = rebalance::rebalance(&mut self.vtree, &mut self.gtree, depth_evict);
```

The `depth_evict` temporary exists solely to satisfy the borrow checker — it is *always*
`self.gtree.live_depth_evict` and carries no independent information.  Exposing this as a
thin `pub(crate)` method on `GvGraph` eliminates the boilerplate, makes the intent
self-documenting, and keeps callers free of `rebalance` module internals.

---

## Why this is the right change

### 1. All call sites are already inside `&mut self` GvGraph methods

Every production invocation of the free function appears inside an `impl GvGraph` method
(`observe`, `post_decay_repair`, `check_evictions`).  There is no call site that is not
ultimately owned by a `GvGraph`.

### 2. `depth_evict` is never a free variable

The `depth_evict` parameter is read-only and is always `self.gtree.live_depth_evict`.
Passing it as an explicit argument to a function that already receives `&mut self.gtree` is
redundant noise.  Making the method pull it from `self` is strictly simpler.

### 3. Consistent with the existing design pattern

The comment at the top of `gv_graph.rs` already describes the design intent:

> *"GvGraph methods are implemented as a combined owner in this file, but the core algorithm
> behaviors are defined in submodules under `src/graph/algorithm/`"*

The free function body stays in `rebalance.rs` untouched.  The new method is a thin
wrapper — exactly the pattern followed by `observe`, `decay`, `handle_legacy_promotes`, and
every other entry-point method on `GvGraph`.

### 4. Reduces coupling between callers and rebalance internals

After the change, `observe.rs`, `decay.rs`, and `budget.rs` no longer need to import the
`rebalance` module directly for the rebalance step.  Each file continues to import whatever
other symbols it needs from `rebalance` (e.g. `find_violated_nodes`), so the import is
not removed blindly — only the dependency on the call convention changes.

### 5. `Inspectable` bound stays local to one impl block

`rebalance` requires `V: Accumulator + Inspectable`.  All three callers already live inside
`impl<C, V: Accumulator + Inspectable, N> GvGraph<C, V, N>` blocks — `observe.rs` and
`budget.rs` explicitly, `decay.rs` via `post_decay_repair` which is in the same impl block.
The new wrapper method therefore requires no new trait bounds anywhere.

---

## What is NOT changing

- The free function `rebalance::rebalance` itself — body, signature, and tests are unchanged.
- The `rebalance.rs` module structure remains intact.
- No public API surface changes.
- `handle_iteration_limit` and `resolve` remain as free functions.

---

## Step-by-step guide

### Step 1 — Add `rebalance_vtree` to `budget.rs` ✅

`budget.rs` already hosts the `impl<C, V: Accumulator + Inspectable, N> GvGraph<C, V, N>`
block that contains `handle_legacy_promotes`, `adjust_depth_gates`, and `check_evictions`.
Add the new thin wrapper at the top of that block:

```rust
pub(crate) fn rebalance_vtree(&mut self) -> Vec<GNodeId> {
    let depth_evict = self.gtree.live_depth_evict;
    rebalance::rebalance(&mut self.vtree, &mut self.gtree, depth_evict)
}
```

### Step 2 — Update `observe.rs` ✅

Replace:
```rust
let depth_evict = self.gtree.live_depth_evict;
let new_gnodes = rebalance::rebalance(&mut self.vtree, &mut self.gtree, depth_evict);
```
With:
```rust
let new_gnodes = self.rebalance_vtree();
```

### Step 3 — Update `decay.rs` (`post_decay_repair`) ✅

Same substitution as Step 2.

### Step 4 — Update `budget.rs` (`check_evictions`) ✅

Same substitution as Step 2.

### Step 5 — Verify ✅

```bash
./scripts/verify.sh
```

All four sub-steps must pass: `cargo fmt --check`, `clippy-strict`, `cargo test --all-features`,
`cspell-check`.

---

## Progress tracker

| Step | Description                              | Status |
|------|------------------------------------------|--------|
| 1    | Add `rebalance_vtree` to `budget.rs`     | ✅ Done |
| 2    | Update `observe.rs`                      | ✅ Done |
| 3    | Update `decay.rs`                        | ✅ Done |
| 4    | Update `budget.rs` call site             | ✅ Done |
| 5    | `./scripts/verify.sh` passes             | ✅ Done |
