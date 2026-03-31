# Refactoring Plan: Extract `GvCore<C, V, N>` from `GvGraph`

## Status: Draft — awaiting review

---

## Motivation

Several free functions and `GvGraph` methods operate exclusively on the `(gtree, GTree)` + `(vtree, VTree)` pair, without needing `config` or the plateau `tracker`. Their signatures expose the coupling explicitly:

```rust
pub fn rebalance<C, V, const N>(vtree: &mut VTree<V>, gtree: &mut GTree<C, V, N>, depth_evict: u32)
pub fn legacy_promote<C, V, const N>(vtree: &mut VTree<V>, gtree: &mut GTree<C, V, N>, c: VNodeId)
pub fn alloc_v_entry<C, V, const N>(vtree: &mut VTree<V>, gtree: &mut GTree<C, V, N>, gnode: GNodeId)
pub fn resolve<C, V, const N>(tree: &mut VTreeMutContext<'_, V>, gtree: &mut GTree<C, V, N>, ...)
```

`depth_evict` in `rebalance` is never a real free variable — it is always `gtree.live_depth_evict`. The dual-tree pair is the natural owner of these operations, yet it has no name in the type system.

Extracting a named `GvCore<C, V, N>` struct gives the pair an identity, moves the functions to where they semantically belong, enables independent unit-testing without constructing a full `GvGraph`, and removes the borrow-splitting boilerplate at every call site.

---

## New type

```rust
// src/graph/core.rs  (new file)
pub(crate) struct GvCore<C: Coordinate, V: Accumulator, const N: u32> {
    pub(crate) gtree: GTree<C, V, N>,
    pub(crate) vtree: VTree<V>,
}
```

`GvGraph` becomes:

```rust
pub struct GvGraph<C, V, const N: u32, T = DefaultTracker<C, V>> {
    pub(crate) core:    GvCore<C, V, N>,   // ← replaces individual gtree/vtree fields
    pub(crate) config:  Config<V>,
    pub(crate) tracker: T,
}
```

Every existing `self.gtree` / `self.vtree` in `GvGraph` impl blocks and algorithm submodules becomes `self.core.gtree` / `self.core.vtree`.

---

## Free functions that move to `GvCore` methods

### 1. `rebalance` → `GvCore::rebalance`

**Current location:** `src/graph/algorithm/rebalance.rs`

**Current signature:**
```rust
pub fn rebalance<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    vtree: &mut VTree<V>,
    gtree: &mut GTree<C, V, N>,
    depth_evict: u32,
) -> Vec<GNodeId>
```

**New signature:**
```rust
// on impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvCore<C, V, N>
pub(crate) fn rebalance(&mut self) -> Vec<GNodeId>
// depth_evict is self.gtree.live_depth_evict — no longer a parameter
```

**Call-site changes:**
- `budget.rs`: `self.rebalance_vtree()` → `self.core.rebalance()` (the `rebalance_vtree` wrapper on `GvGraph` added in the previous refactor is also removed)
- `observe.rs`: same
- `decay.rs`: same
- Tests in `rebalance.rs` that call `rebalance(&mut g.vtree, &mut g.gtree, depth_evict)` directly use `g.core.rebalance()` or construct a `GvCore` directly instead of a full `GvGraph`

---

### 2. `legacy_promote` → `GvCore::legacy_promote`

**Current location:** `src/graph/algorithm/promote.rs`

**Current signature:**
```rust
pub fn legacy_promote<C: Coordinate, V: Accumulator, const N: u32>(
    vtree: &mut VTree<V>,
    gtree: &mut GTree<C, V, N>,
    c: VNodeId,
) -> GNodeId
```

**New signature:**
```rust
// on impl<C: Coordinate, V: Accumulator, const N: u32> GvCore<C, V, N>
pub(crate) fn legacy_promote(&mut self, c: VNodeId) -> GNodeId
```

**Call-site changes:**
- `rebalance/resolve.rs` (`resolve_path_b`): currently calls `legacy_promote(tree.vtree, gtree, c)` where `tree` is a `&mut VTreeMutContext` and `gtree` is a `&mut GTree`. With `GvCore` this becomes `core.legacy_promote(c)` — but see the **borrow-splitting note** below.
- `promote.rs` tests: `legacy_promote(&mut graph.vtree, &mut graph.gtree, c)` → `graph.core.legacy_promote(c)`

**Borrow-splitting note:** `resolve_path_b` currently holds a `&mut VTreeMutContext` (which itself holds `&mut VTree`) and a separate `&mut GTree`. Moving to a `&mut GvCore` argument would unify those borrows, which is the goal. The `VTreeMutContext` is then constructed inside `legacy_promote` for the internal borrow-splitting it needs, rather than being threaded in from the caller.

---

### 3. `alloc_v_entry` → `GvCore::alloc_v_entry`

**Current location:** `src/graph/algorithm/split/helpers.rs`

**Current signature:**
```rust
pub(super) fn alloc_v_entry<C: Coordinate, V: Accumulator, const N: u32>(
    vtree: &mut VTree<V>,
    gtree: &mut GTree<C, V, N>,
    gnode: GNodeId,
) -> VNodeId
```

**New signature:**
```rust
// on impl<C: Coordinate, V: Accumulator, const N: u32> GvCore<C, V, N>
pub(crate) fn alloc_v_entry(&mut self, gnode: GNodeId) -> VNodeId
```

**Call-site changes:**
- `split/helpers.rs` (`allocate_split_children`): `alloc_v_entry(&mut self.vtree, &mut self.gtree, id)` → `self.core.alloc_v_entry(id)`
- `promote.rs` tests: the test for `legacy_promote` constructs `GvGraph` to get tree pair access; with `GvCore` it can use `GvCore` directly

---

### 4. `resolve` and `resolve_path_b` — conditional

**Current location:** `src/graph/algorithm/rebalance/resolve.rs`

**Current signatures:**
```rust
pub fn resolve<C, V, const N>(tree: &mut VTreeMutContext<'_, V>, gtree: &mut GTree<C, V, N>, c: VNodeId, depth_evict: u32) -> Option<GNodeId>
fn resolve_path_b<C, V, const N>(tree: &mut VTreeMutContext<'_, V>, gtree: &mut GTree<C, V, N>, ...) -> Option<GNodeId>
```

These take a `VTreeMutContext` (a borrow-splitting wrapper over `&mut VTree`) rather than `&mut VTree` directly, because the resolve logic both mutates structural nodes and pushes to the violation queue simultaneously — operations that would conflict on a plain `&mut VTree` borrow-split.

**Option A (recommended for this plan):** keep `resolve` and `resolve_path_b` as free functions with their current signatures. `GvCore::rebalance` constructs a `VTreeMutContext { vtree: &mut self.vtree }` internally and passes it alongside `&mut self.gtree`, as `rebalance` already does today. No change to these two functions.

**Option B (deferred):** convert them to `GvCore` methods by taking `&mut self` and constructing `VTreeMutContext` inside. This flattens the call chain further but is lower priority. Consider it as a follow-up once the rest of the plan lands.

---

## `VTree::remove_leaf(&mut GTree)` — out of scope

`evict.rs` calls `self.vtree.remove_leaf(&mut self.gtree, v_id)`. This is a method on `VTree` that reaches into `GTree`, which is a symmetrical design smell in the other direction. It is out of scope for this plan and should be addressed separately.

---

## What does NOT move

| Item | Reason |
|------|--------|
| `standard_promote` | Only touches `VTree` — stays as a free function in `promote.rs` |
| `skip_promote` | Only touches `VTree` — stays as a free function in `promote.rs` |
| `contract` | Only touches `VTree` — stays as a free function in `rebalance.rs` |
| `is_violated`, `max_uncle_intensity`, `find_violated_nodes` | Only touch `VNodeTree` — stay as free functions |
| `handle_iteration_limit` | Diagnostic helper, only reads `VNodeTree` |
| `GvGraph` algorithm methods (`observe`, `decay`, `extract`, `sample`, ...) | Need `self.config` and/or `self.tracker` — stay on `GvGraph`, field paths become `self.core.*` |

---

## File layout after refactoring

```
src/graph/
  core.rs             ← NEW: GvCore struct + alloc_v_entry, rebalance, legacy_promote methods
  gv_graph.rs         ← MODIFIED: core: GvCore field; self.gtree → self.core.gtree throughout
  algorithm/
    budget.rs         ← MODIFIED: self.rebalance_vtree() → self.core.rebalance()
    observe.rs        ← MODIFIED: field paths + rebalance call
    decay.rs          ← MODIFIED: field paths + rebalance call
    evict.rs          ← MODIFIED: field paths
    extract.rs        ← MODIFIED: field paths
    split/helpers.rs  ← MODIFIED: alloc_v_entry call → self.core.alloc_v_entry(); free fn removed
    promote.rs        ← MODIFIED: legacy_promote body moves to GvCore; free fn becomes a thin delegate or is removed
    rebalance.rs      ← MODIFIED: rebalance body moves to GvCore; free fn becomes thin delegate or removed
    rebalance/
      resolve.rs      ← UNCHANGED (Option A)
```

---

## Impact on tests

- Tests in `rebalance.rs` that call the free `rebalance` function directly can switch to constructing a `GvCore` (via a `GvCore::new` or by extracting from a `GvGraph`) rather than a full `GvGraph`. This is the testability improvement mentioned in the motivation.
- Tests in `promote.rs` for `legacy_promote` that access `graph.vtree` / `graph.gtree` directly will access `graph.core.vtree` / `graph.core.gtree`.
- All existing pass/fail semantics are unchanged — this is a pure structural refactor.

---

## Step-by-step implementation guide

| Step | Action | Files touched |
|------|--------|---------------|
| 1 | Create `src/graph/core.rs` with `GvCore` struct and a `GvCore::new` constructor mirroring the field setup in `build_core` | core.rs |
| 2 | Update `build_core` in `gv_graph.rs` to return a `GvCore` instead of separate `(GTree, VTree)` | gv_graph.rs |
| 3 | Replace `gtree` and `vtree` fields in `GvGraph` with `core: GvCore<C, V, N>` and update all constructors | gv_graph.rs |
| 4 | Mechanical field-path update: `self.gtree` → `self.core.gtree`, `self.vtree` → `self.core.vtree` across all algorithm files | all algorithm/*.rs |
| 5 | Move `rebalance` body to `GvCore::rebalance`; remove the `GvGraph::rebalance_vtree` wrapper from `budget.rs`; update all call sites | rebalance.rs, budget.rs, observe.rs, decay.rs |
| 6 | Move `legacy_promote` body to `GvCore::legacy_promote`; keep or remove the free function depending on whether any call sites need it | promote.rs, rebalance/resolve.rs |
| 7 | Move `alloc_v_entry` body to `GvCore::alloc_v_entry`; remove the free function | split/helpers.rs |
| 8 | Run `./scripts/verify.sh` — fix any errors | — |
| 9 | Update test helpers in `rebalance.rs` and `promote.rs` to use `GvCore` where applicable | rebalance.rs, promote.rs |
| 10 | Run `./scripts/verify.sh` again — all checks must pass | — |

---

## Progress tracker

| Step | Status |
|------|--------|
| 1. Create `GvCore` struct | ✅ Done — `bb15b61` |
| 2. Update `build_core` return | ✅ Done — `bb15b61` |
| 3. Replace fields in `GvGraph` | ✅ Done — `bb15b61` |
| 4. Mechanical field-path update | ✅ Done — `bb15b61` |
| 5. Move `rebalance` to `GvCore` | ✅ Done — `d083d5e` (delegates to free function; wrapper removed) |
| 6. Move `legacy_promote` to `GvCore` | ⏭ Skipped — `dead_code` lint fires: only call site is in `rebalance/resolve.rs` which uses separate `(vtree, gtree)` borrows; needs Option B (resolve → GvCore) deferred |
| 7. Move `alloc_v_entry` to `GvCore` | ✅ Done — `8cf239a` (body moved; free function removed) |
| 8. First verify run | ✅ Passed after each step |
| 9. Update tests to use `GvCore` directly | ⏭ Skipped — production `dead_code` constraint prevents test-only GvCore methods |
| 10. Final verify run | ✅ Passing |
