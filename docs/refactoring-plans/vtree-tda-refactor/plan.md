# Refactoring Plan: Route All V-tree Mutations Through `VTree`

## Goal

Apply the **Tell Don't Ask** principle to V-tree mutations.
Today, algorithm code (`promote.rs`, `rebalance.rs`, `split.rs`)
reaches past `VTree` to operate directly on its internal `Arena<VNode<V>>`.
After this refactor, every V-tree mutation goes through a `VTree` method,
and the raw arena is no longer part of any public or algorithm-facing API.

## Reference

- [vnodes-arena-mut-functions.md](./vnodes-arena-mut-functions.md) —
  full list of functions identified as TDA violations.

## Quality Gate (run before every commit)

```bash
cargo test --all-targets --all-features
bash scripts/cspell-check.sh
bash scripts/clippy-strict.sh
```

All three must pass green before committing a step.

---

## Phases

### Phase 1 — Audit & baseline

**Goal**: understand what we have before touching anything.

- [ ] **Step 1.1** Run the quality gate and record the baseline result
  (tests, cspell, clippy all green).
- [ ] **Step 1.2** Read `VTreeMutContext` in
  `src/graph/algorithm/rebalance/context.rs` and list every field and caller.
- [ ] **Step 1.3** Confirm `VTree` already wraps every `mutation.rs` function
  via `VTree::add_structural_child`, `::replace_structural_child`, etc.
  Document any gaps (e.g. `set_has_evictable` has no `VTree` wrapper yet).

---

### Phase 2 — Add missing `VTree` wrapper methods

**Goal**: give `VTree` a complete interface so algorithm code has something to
call in later phases. No caller changes yet.

- [ ] **Step 2.1** Add `pub(crate) fn set_has_evictable(&mut self, id: VNodeId, flag: bool)`
  to `VTree` in `src/tree/vtree.rs`.
  Body: delegates to `set_has_evictable(&mut self.nodes, id, flag)`.
  Run quality gate.

- [ ] **Step 2.2** Add `pub(crate) fn recompute_and_sync(&mut self, id: VNodeId)`
  to expose `recompute_and_sync_parent_slot` as a `VTree` method.
  Run quality gate.

- [ ] **Step 2.3** Demote `pub fn propagate_evictable_flags` to `pub(crate)`.
  Update the one import in `rebalance.rs` to still compile.
  Run quality gate.

- [ ] **Step 2.4** Demote `pub fn recompute_and_sync_parent_slot` to `pub(crate)`.
  Update the import in `promote.rs` to still compile.
  Run quality gate.

> Commit: `refactor(vtree): add missing VTree wrapper methods and tighten visibility`

---

### Phase 3 — Update `promote.rs` to use `VTree` wrappers

**Goal**: replace raw-arena calls in `promote.rs` with `VTree` method calls.
This is purely a caller update — no logic changes.

Files touched: `src/graph/algorithm/promote.rs`.

- [ ] **Step 3.1** Change `standard_promote` signature:
  `vnodes: &mut Arena<VNode<V>>` → `vtree: &mut VTree<V>`.
  - Replace `propagate_evictable_flags(vnodes, p)` with `vtree.propagate_evictable(p)`.
  - Replace direct arena access (`vnodes.get`, `vnodes.get_mut`, `vnodes.dealloc`)
    with `vtree.nodes.get` / … (same internal access — the arena field stays
    accessible within the crate for now).
  - Update call-sites of `standard_promote` (find via `grep`).
  - Run quality gate.

- [ ] **Step 3.2** Same treatment for `skip_promote`.
  Run quality gate.

- [ ] **Step 3.3** Same treatment for `legacy_promote`.
  Its signature becomes `(vtree: &mut VTree<V>, gnodes: &mut Arena<GNode<C, V>>, c: VNodeId)`.
  - Replace `replace_child_in_parent(vnodes, ...)` with `vtree.replace_structural_child(...)`.
  - Replace `set_entry_flags(vnodes, ...)` with `vtree.set_entry_flags(...)`.
  - Replace `recompute_and_sync_parent_slot(vnodes, p)` with `vtree.recompute_and_sync(p)`.
  - Replace `propagate_evictable_flags(vnodes, x)` with `vtree.propagate_evictable(x)`.
  - Update call-sites.
  - Run quality gate.

- [ ] **Step 3.4** Remove now-unused imports in `promote.rs`
  (`propagate_evictable_flags`, `recompute_and_sync_parent_slot`,
  `replace_child_in_parent`, `set_entry_flags`).
  Run quality gate.

> Commit: `refactor(promote): route all V-tree mutations through VTree methods`

---

### Phase 4 — Update `rebalance.rs` (`contract`) to use `VTree`

**Goal**: `contract` currently takes `vnodes: &mut Arena<VNode<V>>`.
Change it to `vtree: &mut VTree<V>`.

Files touched: `src/graph/algorithm/rebalance.rs`,
`src/graph/algorithm/rebalance/resolve.rs`,
`src/graph/algorithm/split.rs` (calls `contract` via `self.vtree.nodes`).

- [ ] **Step 4.1** Update `contract` signature to `(vtree: &mut VTree<V>, p: VNodeId)`.
  Replace `propagate_evictable_flags(vnodes, p)` with `vtree.propagate_evictable(p)`.
  Run quality gate.

- [ ] **Step 4.2** Update all `contract(vnodes, ...)` call-sites:
  - `resolve.rs` currently calls `contract(vnodes, ...)` where `vnodes`
    comes from `VTreeMutContext::vnodes`. Change to pass `tree.vtree` once
    `VTreeMutContext` holds it (done in Phase 5), or temporarily accept a
    transitional call `contract(&mut *tree.vnodes, ...)` if Phase 5 is later.
  - `split.rs`: `contract(&mut self.vtree.nodes, p_id)` → `contract(&mut self.vtree, p_id)`.
  - Run quality gate.

- [ ] **Step 4.3** Remove `use crate::tree::vtree::propagate_evictable_flags`
  from `rebalance.rs`.
  Run quality gate.

> Commit: `refactor(rebalance): contract takes &mut VTree instead of raw arena`

---

### Phase 5 — Restructure `VTreeMutContext`

**Goal**: this is the highest-leverage structural change. Replace the two raw
fields with a single `&mut VTree<V>`.

Files touched: `src/graph/algorithm/rebalance/context.rs`,
`src/graph/algorithm/rebalance/resolve.rs`, and every consumer of
`VTreeMutContext`.

- [ ] **Step 5.1** Change `VTreeMutContext`:

  ```rust
  // Before
  pub struct VTreeMutContext<'a, V: Accumulator> {
      pub vnodes: &'a mut Arena<VNode<V>>,
      pub violations: &'a mut Vec<VNodeId>,
  }

  // After
  pub struct VTreeMutContext<'a, V: Accumulator> {
      pub vtree: &'a mut VTree<V>,
  }
  ```

  Update the import in `context.rs` to bring in `VTree`.

- [ ] **Step 5.2** Fix all construction sites of `VTreeMutContext`
  (find with `grep VTreeMutContext`):
  change `{ vnodes: &mut ..., violations: &mut ... }` to `{ vtree: &mut self.vtree }`.

- [ ] **Step 5.3** In `resolve.rs`, replace every
  `tree.vnodes` with `tree.vtree.nodes` and every
  `tree.violations` with `tree.vtree.violations`.
  Run quality gate after each function updated.

- [ ] **Step 5.4** Run the full quality gate. Fix any remaining references.

> Commit: `refactor(rebalance): VTreeMutContext holds &mut VTree instead of raw fields`

---

### Phase 6 — Update `split/helpers.rs`

**Goal**: remove the raw-arena parameters from `alloc_v_entry` and
`alloc_v_structural_2`.

Files touched: `src/graph/algorithm/split/helpers.rs`,
`src/graph/algorithm/split.rs`.

- [ ] **Step 6.1** Move `alloc_v_structural_2` into `VTree` as a method:

  ```rust
  pub(crate) fn alloc_structural_2(&mut self, a: VNodeId, b: VNodeId) -> VNodeId
  ```

  Body is the current function body using `self.nodes`.
  Update the one call site in `split.rs` to `self.vtree.alloc_structural_2(...)`.
  Remove the free function.
  Run quality gate.

- [ ] **Step 6.2** Update `alloc_v_entry` to take `(vtree: &mut VTree<V>, gnodes: &mut Arena<GNode<C,V>>, gnode: GNodeId)`.
  Update its two call-sites in `helpers.rs`.
  Run quality gate.

> Commit: `refactor(split): move VNode allocation helpers to VTree`

---

### Phase 7 — Collapse `vtree/mutation.rs`

**Goal**: since all external callers now go through `VTree` methods,
`mutation.rs` is only called by `VTree` itself. Inline its bodies directly
into the `VTree` methods and delete the submodule.

Files touched: `src/tree/vtree.rs`, `src/tree/vtree/mutation.rs`.

- [ ] **Step 7.1** Inline `replace_child_in_parent` body into
  `VTree::replace_structural_child`. Confirm no other caller exists.

- [ ] **Step 7.2** Inline `add_child_to_structural` body into
  `VTree::add_structural_child`.

- [ ] **Step 7.3** Inline `remove_child_from_structural` body into
  `VTree::remove_structural_child`.

- [ ] **Step 7.4** Inline `set_entry_flags` body into `VTree::set_entry_flags`.

- [ ] **Step 7.5** Inline `set_has_evictable` body into `VTree::set_has_evictable`.
  Note: `set_has_evictable` is still used by `propagate_evictable_flags` (a
  private free function). Keep a private `fn set_has_evictable_inner` in the
  module, or pass the flag directly, whichever clippy/borrow-checker allows.

- [ ] **Step 7.6** Delete `src/tree/vtree/mutation.rs`.
  Remove `mod mutation;` and the `pub use mutation::...` block from `vtree.rs`.
  Run quality gate.

> Commit: `refactor(vtree): inline mutation.rs helpers and delete the submodule`

---

### Phase 8 — Final cleanup & documentation update

- [ ] **Step 8.1** Search for any remaining direct `&mut Arena<VNode<V>>`
  parameters in non-private functions across the whole codebase.
  Resolve any stragglers.

- [ ] **Step 8.2** Update the module-level doc comment in `vtree.rs` to
  reflect the new design (no external raw-arena access).

- [ ] **Step 8.3** Update `vnodes-arena-mut-functions.md` in this folder to
  mark all items resolved.

- [ ] **Step 8.4** Run the full quality gate one final time.

> Commit: `docs(vtree): update module docs after TDA refactor`

---

## Progress Summary

| Phase | Status | Commit |
|---|---|---|
| 1 — Audit & baseline | ⬜ not started | — |
| 2 — Add missing VTree wrappers | ⬜ not started | — |
| 3 — Fix `promote.rs` | ⬜ not started | — |
| 4 — Fix `rebalance.rs` (`contract`) | ⬜ not started | — |
| 5 — Restructure `VTreeMutContext` | ⬜ not started | — |
| 6 — Fix `split/helpers.rs` | ⬜ not started | — |
| 7 — Collapse `mutation.rs` | ⬜ not started | — |
| 8 — Final cleanup | ⬜ not started | — |
