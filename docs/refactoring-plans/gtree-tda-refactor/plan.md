# Refactoring Plan: Route G-tree Mutations Through `GTree`

## Goal

Apply the same Tell-Don't-Ask cleanup previously used for `VTree`, but now for
functions that mutate the G-tree through raw arena parameters.

Before this refactor, several cross-tree algorithms accepted
`&mut Arena<GNode<...>>` and reached into the G-node arena directly.
After this refactor, production code mutates G-nodes through `GTree` methods
and passes `&mut GTree<...>` instead of the raw arena.

## Reference

- [gnodes-arena-mut-functions.md](./gnodes-arena-mut-functions.md)

## Quality Gate

```bash
cargo test --lib --all-features
cargo test --test integration --all-features
cargo test --test snapshot_tests --all-features
bash scripts/cspell-check.sh
bash scripts/clippy-strict.sh
```

## Phases

### Phase 1 — Inventory and scope

**Goal**: identify every `&mut Arena<GNode...>` parameter and separate
production APIs from local test scaffolding.

- [x] Search for `&mut Arena<GNode` across `src/**/*.rs`.
- [x] Record every production hit in the inventory doc.
- [x] Mark plateau-only helper functions as test scaffolding, not production API.

### Phase 2 — Add missing `GTree` mutation wrappers

**Goal**: give cross-tree algorithms a `GTree` API they can call instead of
mutating the arena directly.

- [x] Add `GTree::assign_entry`.
- [x] Add `GTree::clear_entry`.
- [x] Add `GTree::allocate_missing_child` for legacy promote.
- [x] Keep legacy-promote count accounting unchanged by leaving node/terminal
  counter updates in the existing batched handling path.

### Phase 3 — Refactor cross-tree helpers

**Goal**: remove raw mutable G-node arena parameters from helper functions.

- [x] Change `alloc_v_entry` to take `&mut GTree<C, V, N>`.
- [x] Change `VTree::remove_leaf` to take `&mut GTree<C, V, N>`.
- [x] Update helper and unit-test call sites.

### Phase 4 — Refactor rebalance / promote pipeline

**Goal**: remove raw mutable G-node arena parameters from the rebalance path.

- [x] Change `legacy_promote` to take `&mut GTree<C, V, N>`.
- [x] Change `resolve_path_b` to take `&mut GTree<C, V, N>`.
- [x] Change `resolve` to take `&mut GTree<C, V, N>`.
- [x] Change `rebalance` to take `&mut GTree<C, V, N>`.
- [x] Update observe / decay / budget / tests to pass `&mut self.gtree`.

### Phase 5 — Final verification and documentation

**Goal**: confirm the production refactor is complete and document the result.

- [x] Verify production `&mut Arena<GNode` signatures are gone.
- [x] Confirm only plateau test helpers still use raw mutable G-node arenas.
- [x] Run the quality gate.
- [x] Write this plan and the inventory document.

## Outcome

### Production signatures removed

- `legacy_promote`
- `resolve_path_b`
- `resolve`
- `rebalance`
- `alloc_v_entry`
- `VTree::remove_leaf`

### Production signatures intentionally left alone

None.

### Remaining raw mutable G-node arena parameters

Only local test helpers in plateau-related test modules still accept
`&mut Arena<GNode<...>>`.

## Progress Summary

| Phase | Status |
|---|---|
| 1 — Inventory and scope | complete |
| 2 — Add missing `GTree` wrappers | complete |
| 3 — Refactor cross-tree helpers | complete |
| 4 — Refactor rebalance / promote pipeline | complete |
| 5 — Final verification and documentation | complete |
