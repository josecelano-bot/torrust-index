# Step 2 — `VTree` Mutation Methods

Sub-plan of [oo-method-migration.md](oo-method-migration.md).

## Goal

Move free functions that only need `&mut VTree<V>` (the nodes arena and the
violations queue) into methods on `VTree`. No algorithmic changes; no `Config`,
`GTree`, or `GvCore` dependency.

## VTree structure recap

```rust
pub struct VTree<V: Accumulator> {
    pub(crate) nodes:      VNodeTree<V>,  // arena + root
    pub(crate) violations: Vec<VNodeId>,  // pending rebalance queue
}
```

All three candidate functions only read / write `VTree` fields and do not touch
anything outside of it.

## Candidates

### A. `contract` — V-tree node merge

| Property | Value |
|---|---|
| Source | `src/graph/algorithm/rebalance.rs:13` |
| Current visibility | `pub` |
| Signature | `pub fn contract<V: Accumulator>(vtree: &mut VTree<V>, p: VNodeId) -> VNodeId` |
| VTree fields | `nodes` only (alloc, dealloc, get, get_mut, propagate\_evictable via VTree method) |
| Config dependency | None |
| Call sites | `resolve.rs` (×4), `split/helpers.rs` (×1) |

### B. `standard_promote` — 2-child structural absorbs into parent

| Property | Value |
|---|---|
| Source | `src/graph/algorithm/promote.rs:19` |
| Current visibility | `pub` |
| Signature | `pub fn standard_promote<V: Accumulator>(vtree: &mut VTree<V>, c: VNodeId)` |
| VTree fields | `nodes` only (get, get_mut, dealloc, propagate\_evictable via VTree method) |
| Config dependency | None |
| Call sites | `resolve.rs` (×1) |

### C. `skip_promote` — entry skips parent and joins grandparent

| Property | Value |
|---|---|
| Source | `src/graph/algorithm/promote.rs:71` |
| Current visibility | `pub` |
| Signature | `pub fn skip_promote<V: Accumulator>(vtree: &mut VTree<V>, c: VNodeId) -> Option<VNodeId>` |
| VTree fields | `nodes` only (get, get_mut, dealloc, propagate\_evictable via VTree method) |
| Config dependency | None |
| Call sites | `resolve.rs` (×2) |

## Scope: what stays in `resolve.rs`

The following private free functions in `resolve.rs` also take `&mut VTree<V>`,
but they are **not** migrated in this step because they additionally depend on
algorithm-layer types (`EscalationContext`, `ViolationQueue`) that live in the
algorithm module. Moving them would create a backward dependency from
`tree::vtree` onto `graph::algorithm`:

| Function | Reason it stays |
|---|---|
| `escalate_contract_parent` | Needs `EscalationContext` + `ViolationQueue` |
| `escalate_try_contract_grandparent` | Same |
| `escalate_skip_promote` | Same |
| `escalate_after_promote` | Same |
| `resolve_try_contract_parent` | Same |

After this step, these private helpers still call `contract(vtree, p)`,
`skip_promote(vtree, c)`, etc. — those calls become `vtree.contract(p)`,
`vtree.skip_promote(c)`, updating naturally at their call sites.

## Tracing spans

`contract`, `standard_promote`, and `skip_promote` contain `tracing::debug_span!`
calls that format nodes using the display helpers `Nd` and `Ch` from
`src/graph/algorithm/fmt.rs`. `fmt.rs` imports `VNodeTree`, so importing `Nd`/`Ch`
inside `vtree/mod.rs` would create a circular dependency.

Resolution: replace the `%Nd(...)` and `%Ch(...)` span fields with `.index()`
plain integers. The span names and `tracing::debug!` messages are preserved; only
the structured field formatting is simplified.

## Files changed

| File | Change |
|---|---|
| `src/tree/vtree/mod.rs` | Add `contract`, `standard_promote`, `skip_promote` methods; move tests from `promote.rs` |
| `src/graph/algorithm/rebalance.rs` | Remove `contract` free function + update imports |
| `src/graph/algorithm/promote.rs` | **Delete** (both functions move; file becomes empty) |
| `src/graph/algorithm/mod.rs` | Remove `pub mod promote;` |
| `src/graph/algorithm/rebalance/resolve.rs` | Remove `promote` import; update 4 call sites to method calls |
| `src/graph/algorithm/split/helpers.rs` | Remove `contract` from import; update 1 call site |

## Ordering within this step

1. Add the three methods to `VTree` (`vtree/mod.rs`) — no callers changed yet.
2. Remove the free functions and update all callers in one pass.
3. Delete `promote.rs`; remove `pub mod promote` from `mod.rs`.
4. Run `./scripts/verify.sh`.

## Tests

- `promote.rs` unit tests (`standard_promote_*`, `skip_promote_*`) move into
  `src/tree/vtree/mod.rs` inside `#[cfg(test)] mod tests`.
- `contract` has no dedicated unit test in `rebalance.rs` (logic is covered by
  integration and snapshot tests); no new test is added in this step.

## Commit plan

```
refactor(vtree): move contract, standard_promote, skip_promote to VTree methods
```

(Single commit: add methods + remove free fns + update callers + delete promote.rs)

## Progress

- [x] `contract` → `VTree::contract` method
- [x] `standard_promote` → `VTree::standard_promote` method
- [x] `skip_promote` → `VTree::skip_promote` method
- [x] `promote.rs` deleted
- [x] All call sites updated (`resolve.rs`, `split/helpers.rs`)
- [x] Tests moved to `vtree/mod.rs`
