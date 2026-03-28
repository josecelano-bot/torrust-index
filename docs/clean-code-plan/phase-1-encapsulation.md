# Phase 1 — Encapsulation

## Goal

Make illegal states unrepresentable and remove the ability for algorithm modules
to bypass node invariants by accessing fields directly.

## Files targeted

- `src/nodes/gnode.rs`
- `src/nodes/vnode.rs`

## Background

Currently, all `GNode` and `VNode` fields are `pub(super)` — visible to every
module inside `nodes/`.  The algorithm modules in `graph/algorithm/` access
these fields through public accessor methods that are already defined.  However,
there is no enforcement: a future contributor can easily reach into the field
directly or write a method that bypasses invariant-checking logic.

Two structural weaknesses remain:

1. **`GNode`/`VNode` fields too visible**: `pub(super)` is better than `pub`,
   but it still allows `nodes/mod.rs` to re-export anything.  The fields should
   be private (`pub(crate)` on methods only).
2. **`Children` uses a runtime `len` field**: The invariant "a structural V-node
   always has 2 or 3 children" is encoded as a `len: u8` with runtime assertions
   rather than as a type.

## Acceptance criteria

- [x] `GNode` fields are `pub(crate)` at most; all mutation goes through methods.
- [x] `VNode` fields are `pub(crate)` at most; all mutation goes through methods.
- [x] `Children` is a proper pair/triple enum — no runtime `len` field.
- [x] All existing tests still pass (`cargo test`).
- [x] No new `pub(crate)` field introductions in algorithm modules.

---

## Tasks

### P1.1 — Audit current field visibility in `GNode` and `VNode`

**Status:** `[x]` done

**Findings (2026-03-28):**
- All `GNode` fields were `pub(super)` (visible within `nodes/` only); no code outside `nodes/gnode.rs` accessed them directly — all reads went through accessor methods and all writes through mutator methods.
- `GNode.parent` is set only at construction; no runtime set_parent is needed.
- All `VNode` fields were `pub(super)`; algorithm modules mutate `VKind` sub-fields through `kind_mut()` (returns `&mut VKind<V>`) — not through raw field access.
- `Children` was already a typed enum (`Pair`/`Triple`) with no `len` field — P1.4 was already complete.

**What to do:**

1. Read `src/nodes/gnode.rs` top-to-bottom and list every `pub(super)` / `pub(crate)` field.
2. For each field, check which modules outside `nodes/` access it through a method
   vs which ones still reach in directly (search for `.entry`, `.left`, `.right`,
   `.parent`, `.lo`, `.hi`, `.sum`, `.own`).
3. Record findings in a comment at the top of this task.  Do not change any code yet.

**Acceptance:** Findings documented; no code changes.

---

### P1.2 — Make `GNode` fields private, add/update typed mutators

**Status:** `[x]` done

**Depends on:** P1.1

**What to do:**

Change each `pub(super)` field in `GNode` to `pub(crate)` (accessor methods)
or fully private (mutation only through methods).

The following mutators should be verified or added:

| Operation                     | Method to add/verify                      |
| ----------------------------- | ----------------------------------------- |
| Link left child               | `GNode::link_left(id: GNodeId)`           |
| Link right child              | `GNode::link_right(id: GNodeId)`          |
| Clear a child pointer         | `GNode::clear_child(id: GNodeId)`         |
| Assign entry V-node           | `GNode::assign_entry(vid: VNodeId)`       |
| Detach entry V-node           | `GNode::detach_entry()`                   |
| Set parent                    | `GNode::set_parent(id: Option<GNodeId>)`  |
| Accumulate own value          | `GNode::set_own(v: V)`                    |
| Update sum (post recompute)   | `GNode::set_sum(v: V)`                    |

Each method must enforce any invariant it is responsible for (e.g.
`assign_entry` could assert the node is currently `Terminal` or `SemiInternal`).

**Acceptance:** `cargo test` passes; no field access outside accessor/mutator methods.

---

### P1.3 — Make `VNode` fields private, add/update typed mutators

**Status:** `[x]` done

**Depends on:** P1.1

**What to do:**

Same approach as P1.2 but for `VNode`:

| Operation                           | Method                                        |
| ----------------------------------- | --------------------------------------------- |
| Set parent                          | `VNode::set_parent(id: Option<VNodeId>)`      |
| Update intensity                    | `VNode::set_intensity(v: V)` (already exists) |
| Set `is_exposed` flag               | `VNode::set_exposed(b: bool)`                 |
| Set `is_evictable` flag             | `VNode::set_evictable(b: bool)`               |
| Set `has_evictable` on structural   | `VNode::set_has_evictable(b: bool)`           |
| Swap structural children collection | `VNode::set_children(c: Children<V>)`         |

**Acceptance:** `cargo test` passes; no field access outside accessor/mutator methods.

---

### P1.4 — Redesign `Children` as a typed pair/triple enum

**Status:** `[x]` done (already implemented before this phase)

**Depends on:** P1.3

**Background:**

Currently `Children` stores children as a fixed `[entry; 3]` array plus a
`len: u8` counter.  The valid values of `len` are 2 and 3; `len=1` is
transient/invalid.  This allows `len=0` or `len=1` to exist without compile-time
rejection.

Design:

```rust
pub enum Children<V> {
    Pair  { ids: [VNodeId; 2], values: [V; 2] },
    Triple{ ids: [VNodeId; 3], values: [V; 3] },
}
```

Benefits:
- Eliminates the `len` field.
- Makes `children.len()` a `const fn` returning 2 or 3.
- Makes `contract` (3 → 2) and `expand` (2 → 3) encode the transition in types.
- Removes runtime `assert!(index < self.len())` guards.

**Steps:**

1. Add the new `Children` enum alongside the existing `Children` struct.
2. Port `impl Children` methods one by one.
3. Update all `match &children { ... }` sites in algorithm modules.
4. Remove the old struct.

**Acceptance:** `cargo test` passes; no `len` field on `Children`.

---

## Commit message template

```
refactor(nodes): <what>

<why — reference this plan and the invariant or issue being addressed>

Part of docs/clean-code-plan/phase-1-encapsulation.md task P1.x.
```
