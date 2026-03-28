# Phase 7 — Nodes Structural Clarity

## Goal

Improve the internal structure of `GNode` and `VNode` so that:

- Every logical operation is a named function with a clear doc comment.
- Pattern-match arms that carry behaviour are extracted into small helpers.
- Duplication between the two types (e.g. child iteration) is resolved.
- The modules can be read top-to-bottom as a coherent API.

This phase is distinct from Phase 1 (Encapsulation), which made fields private
and introduced accessor helpers.  Phase 7 targets **structural clarity and
duplication** in the bodies of existing methods and `impl` blocks, and adds
missing behavioural helpers that are currently inlined at call-sites.

## Files targeted

- `src/nodes/gnode.rs`
- `src/nodes/vnode.rs`
- `src/nodes/mod.rs`

## Complexity baseline (2026-03-28)


| CC  | Cognitive | SLOC | File                |
| --- | --------- | ---- | ------------------- |
|  87 |  17       |  631 | `nodes/vnode.rs`    |
|  75 |  12       |  476 | `nodes/gnode.rs`    |

> **Note:** The low Cognitive scores relative to CC indicate that most CC comes
> from `match` arms in enums — each variant contributes 1 to CC but 0 to
> cognitive load.  The primary work here is not reducing CC but improving how the
> code reads and eliminating inline logic at call-sites.

### Function hotspots
There are no functions with CC > 10 in these files.  The complexity is spread
across many small `match` expressions.  The structural issue is:

1. Logic that belongs on `GNode`/`VNode` is currently written at call-sites in
   `gtree.rs`, `vtree.rs`, `observe.rs`, and `rebalance.rs`.
2. Several match arms in both files are longer than 10 lines and carry
   sub-logic that could be extracted.
3. `GNode` and `VNode` share the concept of "children" but express it
   differently; the iterator types are not unified.

## Target (post-Phase 7)

| Metric                           | Target                                         |
| -------------------------------- | ---------------------------------------------- |
| All `match` arms in node files   | ≤ 5 SLOC each (delegate to named helper or method) |
| No logic duplicated across files | Zero cross-file duplication of child iteration |
| `gnode.rs` and `vnode.rs` readable as an API | Every `pub` item has a doc comment |

---
## Background

`GNode` and `VNode` are the core data types of the codebase.  Their
implementations currently sit at the boundary between "simple data" and
"knows how to operate on itself".  After Phase 1 (encapsulation) they expose a
clean getter surface, but the existing method bodies still contain inlined
match arms with embedded logic.

The goal here is to complete the "smart struct" migration: every logical
operation on a node should live on the node, not at the call-site.

## P7.1 audit notes (2026-03-28)

- `rebalance.rs`: inline structural-child cardinality checks (`children.len()==2/3`) and structural count helper were identified as node-owned shape logic.
- `split.rs`: pre-split 3-child parent guard used inline `VKind::Structural` + `children.len()==3`.
- `observe.rs`: no `children.len()` shape logic found; mostly pure accessor usage and orchestration.
- Extraction target: move child-shape predicates to `VNode` helpers (`child_count`, `is_structural_pair`, `is_structural_triple`).

---

## Tasks

### P7.1 — Audit call-sites for node logic that lives outside the node

**Status:** `[x]` done

**What to do:**

1. Use `grep` to find all uses of `GNode` and `VNode` in files other than
   `nodes/gnode.rs` and `nodes/vnode.rs`.
2. For each use, classify it as:
   - **Pure access** (reads a field or calls a getter) — OK, leave as-is.
   - **Logic that belongs on the node** (e.g. `if let Some(children) = node.children() { children.len() > 0 }`) — candidate for extraction.
3. Document the candidates in a comment at the top of the phase file (or a
   scratch file) before touching any code.

**Acceptance:** Audit list documented; no code changed in this step.

---

### P7.2 — Extract inline child-count logic out of `rebalance.rs` and `observe.rs`

**Status:** `[x]` done

**Depends on:** P7.1

**What to do:**

Call-sites in `rebalance.rs` and `observe.rs` currently re-implement predicates
such as:

```rust
// Example of logic that belongs on GNode:
matches!(node, GNode::Internal { children, .. } if children.len() == 1)
```

Move each such predicate onto `GNode` or `VNode` as a named method.  Reference
helpers already added in Phase 1 (`is_leaf`, `has_children`, `child_ids`) and
add any that are still missing.

Do one commit per helper method.

**Acceptance:** No inline node-logic in `rebalance.rs` or `observe.rs`; all
tests pass.

---

### P7.3 — Normalise child iteration between `GNode` and `VNode`

**Status:** `[x]` done

**Depends on:** P7.2

**What to do:**

`GNode` and `VNode` both expose child identifiers but through different shapes:

- `GNode::child_ids()` returns an `impl Iterator<Item = GNodeId>`.
- `VNode` uses `Children::ids()` which returns a different iterator.

Ensure:
1. Both expose a `child_ids()` method with matching semantics (order: left to
   right, no duplicates).
2. Both expose `child_count() -> usize` as a convenience.
3. Call-sites that currently call `.len()` on a `children` field call
   `child_count()` instead.

**Acceptance:** `child_ids()` and `child_count()` present on both types; no
call-site manually counts children; `cargo test` passes.

---

### P7.4 — Add `split_into` helper on `GNode`

**Status:** `[x]` done

**Depends on:** P7.3

**What to do:**

The split operation in `graph/algorithm/split.rs` constructs new `GNode`
variants by building them directly with struct/variant syntax.  This leaks
construction details outside the type.

Add a `GNode::split_into(left: GNodeId, right: GNodeId) -> GNode` constructor
(or equivalent) so that `split.rs` delegates construction to the node type.

**Acceptance:** `split.rs` does not use `GNode::Internal { .. }` literal syntax
outside of `gnode.rs`; tests pass.

---

### P7.5 — Add `reparent` / `detach_child` helpers on `GNode`

**Status:** `[ ]` not started

**Depends on:** P7.4

**What to do:**

`evict.rs` and `rebalance.rs` manipulate `GNode` children by directly mutating
child lists.  Extract:

| Helper fn name                    | Responsibility                                          |
| --------------------------------- | ------------------------------------------------------- |
| `GNode::detach_child(id)`         | Removes `id` from the children list, returns `self`     |
| `GNode::replace_child(old, new)`  | Replaces `old` child id with `new`, returns `self`      |

**Acceptance:** `evict.rs` and `rebalance.rs` do not directly mutate child
vectors; helpers are used universally; tests pass.

---

### P7.6 — Document every `pub` item in `gnode.rs` and `vnode.rs`

**Status:** `[ ]` not started

**Depends on:** P7.5

**What to do:**

1. Scan both files for `pub fn`, `pub struct`, `pub enum`, `pub type`.
2. Add a `/// <one-line-purpose>` doc comment to any item that lacks one.
3. Add a module-level `//! <description>` doc comment to each file if missing.

**Acceptance:** `cargo doc --no-deps` produces zero warnings for `nodes::gnode`
and `nodes::vnode`.

---

### P7.7 — Add unit tests for all new helpers

**Status:** `[ ]` not started

**Depends on:** P7.6

**What to do:**

For each helper added in P7.2–P7.5, add at least one unit test in the `#[cfg(test)]`
block at the bottom of the relevant file.

Tests should be minimal and focused: construct a node in the relevant state,
call the helper, assert the result.  Name tests `<helper_name>_<scenario>`.

**Acceptance:** Every new helper has at least one test; `cargo test` passes.
