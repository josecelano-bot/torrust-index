# Phase 4 — Query Decomposition

## Goal

Decompose the recursive `decompose_basis` function and the high-complexity query
module into clearly-named, individually-testable units.

## File targeted

- `src/graph/algorithm/query.rs`

## Complexity baseline (2026-03-28)

| CC  | Cognitive | SLOC | Function          |
| --- | --------- | ---- | ----------------- |
|  20 |  40       |   90 | `decompose_basis` |

File aggregate: **CC=90, Cog=68, 628 SLOC**.

## Target (post-Phase 4)

| Metric            | Before | Target |
| ----------------- | -----: | -----: |
| File CC           |     90 |  ≤ 50  |
| File Cognitive    |     68 |  ≤ 35  |
| Max function CC   |     20 |  ≤ 8   |
| Max function Cog  |     40 |  ≤ 15  |

---

## Background

`query.rs` implements tree-range queries (`get`, `range_sum`, `contour_range`,
`contour_range_energy`) plus the private `decompose_basis` recursive descent
that all of them rely on.

`decompose_basis` (CC=20, Cog=40) packs three logically distinct cases into a
single function body:

1. **Out-of-range guard** — early return when the query interval does not
   intersect the node interval.
2. **Full-coverage fast path** — when the node is wholly inside the query
   interval, push directly and return.
3. **Asymmetric-child boundary thatch** — special case when exactly one child
   is absent.
4. **Recursive descent** — recurse left/right or produce a boundary thatch
   tile when a child is absent.

The high Cognitive score (40 vs CC=20) signals the cases are deeply nested
rather than linearly arranged.  Flattening the nesting and giving each case
a name eliminates the surprise.

---

## Tasks

### P4.1 — Annotate `decompose_basis` case structure

**Status:** `[x]` done

**What to do:**

1. Read `decompose_basis` top-to-bottom.
2. Label each of the four cases above with a `// Case N:` comment.
3. For the recursive-descent case (case 4), document which `basis.push` calls
   are for "absent child" tiles vs "found child" propagation.
4. Do not change any logic — annotation only.

**Acceptance:** Annotations committed; `cargo test` passes; no logic change.

---

### P4.2 — Extract case predicates and leaf pushes

**Status:** `[x]` done

**Depends on:** P4.1

**What to do:**

Extract the case-specific logic into private helpers so that `decompose_basis`
reads as a pure dispatcher:

| Helper fn name                    | Responsibility                                                    |
| --------------------------------- | ----------------------------------------------------------------- |
| `interval_is_out_of_range`        | Returns `true` if the query does not overlap the node             |
| `interval_is_fully_covered`       | Returns `true` if the node is wholly inside the query             |
| `is_asymmetric_pair`              | Returns `true` when exactly one child is absent                   |
| `push_full_coverage_element`      | Constructs and pushes a full-coverage `BasisElement`              |
| `push_boundary_thatch_element`    | Constructs and pushes a boundary-thatch `BasisElement`            |

After extraction, `decompose_basis` should be ≤ 30 SLOC with no inline
`BasisElement { ... }` struct literals — each push is delegated to a named
helper.

**Acceptance:** `decompose_basis` CC ≤ 8, Cog ≤ 15; `cargo test` passes.

---

### P4.3 — Clarify `range_sum_inner` boundary handling

**Status:** `[x]` done

**Depends on:** P4.2

**What to do:**

`range_sum_inner` mirrors the structure of `decompose_basis` without the plateau
context.  Apply the same clarification:

1. Extract `interval_intersects`/`interval_covers` predicates (reuse if already
   extracted in P4.2).
2. Replace inline `if query_lo > x { query_lo } else { x }` with
   `lo.max(query_lo)` / `hi.min(query_hi)` to reduce noise.

**Acceptance:** `range_sum_inner` CC ≤ 6; `cargo test` passes.

---

### P4.4 — Extract `contour_range` helper: `trim_to_query`

**Status:** `[ ]` not started

**Depends on:** P4.3

**What to do:**

Both `contour_range` and `contour_range_energy` call a pattern like
`range.start_bound()` / `range.end_bound()` to normalize an arbitrary
`RangeBounds<C>` into a `(C, C)` pair.  Extract this into:

```rust
fn clamp_range<R: std::ops::RangeBounds<C>>(range: R, domain: (C, C)) -> (C, C)
```

**Acceptance:** No duplicated range-clamping logic; `cargo test` passes.

---

### P4.5 — Unit tests for extracted query helpers

**Status:** `[ ]` not started

**Depends on:** P4.4

**What to do:**

Add unit tests in `query.rs`'s `#[cfg(test)]` module:

- `decompose_basis` on a single-node graph: full-coverage → 1 element.
- `decompose_basis` on a 2-node graph where only the left half matches.
- `range_sum_inner` on a linear tree: correct sum for an exact-boundary and a
  mid-split query.
- `clamp_range` edge cases: open/closed/unbounded ends.

**Acceptance:** ≥ 4 new tests; `cargo test` passes.

---

## Commit message template

```
refactor(query): <what>

<why — reference cognitive complexity numbers and the specific helper extracted>

Part of docs/clean-code-plan/phase-4-query.md task P4.x.
```
