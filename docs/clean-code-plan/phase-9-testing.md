# Phase 9 — Test Coverage

## Goal

Raise the line coverage of all production files to **≥ 90 %**, eliminating
uncovered branches and code paths that give no confidence under test.

The secondary goal is to ensure that the newly extracted helper functions from
Phases 1–8 each have at least one direct unit test.

## Files targeted (below 90 % line coverage)

- `src/graph/algorithm/plateau/` (combined: 71.82 %)
- `src/diagnostics/invariants.rs` (76.53 %)
- `src/graph/algorithm/evict.rs` (77.09 %)
- `src/graph/algorithm/rebalance.rs` (78.65 %)
- `src/diagnostics/diagnostic.rs` (84.87 %)
- `src/graph/algorithm/query.rs` (84.22 %)
- `src/graph/algorithm/budget.rs` (87.97 %)
- `src/spatial/contour_range.rs` (87.50 %)

## Complexity baseline (2026-03-28)

| File                                        | Line Cover | Fn Cover |
| ------------------------------------------- | ---------: | -------: |
| `graph/algorithm/plateau/` (combined)       |     71.82% |   80.65% |
| `diagnostics/invariants.rs`                 |     76.53% |   95.65% |
| `graph/algorithm/evict.rs`                  |     77.09% |   81.82% |
| `graph/algorithm/rebalance.rs`              |     78.65% |   91.94% |
| `diagnostics/diagnostic.rs`                 |     84.87% |   96.30% |
| `graph/algorithm/query.rs`                  |     84.22% |   97.62% |
| `graph/algorithm/budget.rs`                 |     87.97% |   92.31% |
| `spatial/contour_range.rs`                  |     87.50%  |  100.00% |

> All other files are at ≥ 90 % line coverage in the baseline.

## Target (post-Phase 9)

| Metric                     | Before | Target |
| -------------------------- | -----: | -----: |
| Minimum line coverage      |  71.82%|  ≥ 90 % |
| Uncovered functions        |  19.35% of plateau fns | 0 %  |
| New helpers from Ph 1–8    |  0 direct tests | ≥ 1 each |

---

## Prerequisites

Install `cargo-llvm-cov` if not present:

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

Run coverage before and after each task:

```bash
cargo llvm-cov --workspace --html --output-dir target/coverage
# Open target/coverage/index.html to identify uncovered lines
```

---

## Background

Thetest suite currently passes at 418 unit tests + 10 integration tests +
4 snapshot tests.  However, coverage analysis shows that the most complex parts
of the codebase are the least tested:

- **`plateau/dynamic_tracker.rs`** — the highest-CC file — is below 72%.  Many
  of the `on_evict` and `normalize` paths are only exercised indirectly through
  the integration tests, not directly.
- **`evict.rs`** — the 181-line `evict_tip` function is not fully exercised.
  Missing branches are around the edge cases of leaf removal when a parent becomes
  an only-child.
- **`rebalance.rs`** — the `escalate_after_promote` function's edge-case branches
  (empty-parent, depth-overflow) are untested.

Coverage work is _most effectively done after_ the decomposition phases (1–8):
smaller extracted helpers are easier to test in isolation.  However this phase
is listed last to reflect priority, not sequencing — if a helper in Phases 2–6
can be tested immediately after extraction, do it there and mark the relevant
P9 sub-task done early.

---

## Tasks

### P9.1 — Establish coverage baseline report

**Status:** `[ ]` not started

**What to do:**

1. Run `cargo llvm-cov --workspace --html --output-dir target/coverage`.
2. Open the HTML report and capture per-file line coverage numbers.
3. Update the table at the top of this file with the actual numbers (they may
   differ from the 2026-03-25 snapshot above if Phases 1–8 added tests).
4. Identify the top 10 uncovered line ranges (file + line range from coverage
   report).

**Acceptance:** Updated baseline table committed; coverage report available
locally.

---

### P9.2 — Add unit tests for `plateau/dynamic_tracker.rs`

**Status:** `[ ]` not started

**Depends on:** P9.1, P2.2 (prefer running after Phase 2 decomposition)

**Current coverage:** 71.82% (combined plateau module).

**What to do:**

Work through the uncovered line ranges identified from the HTML report.  The
expected gaps are:

1. The "eviction of a plateau with no remaining children" branch in `on_evict`.
2. The "plateau has a single-element basis" early-return in `normalize`.
3. The `repair_p_i4` fall-through case when no repair is needed.

For each uncovered branch:
1. Write a minimal `GvGraph` setup that forces the branch to be taken.
2. Assert observable state (the plateau structure, the G-tree, or the returned
   value) after the operation.
3. Name tests `<function>_<branch_description>`.

Suggested test helpers to reduce boilerplate:

```rust
fn graph_with_n_observations(n: usize) -> GvGraph<...> { ... }
fn assert_plateau_count(graph: &GvGraph<...>, expected: usize) { ... }
```

**Acceptance:** `plateau/` combined line coverage ≥ 90 %; `cargo test` passes.

---

### P9.3 — Add unit tests for `evict.rs`

**Status:** `[ ]` not started

**Depends on:** P9.1, P6.3 (prefer running after Phase 6 decomposes `evict_tip`)

**Current coverage:** 77.09%.

**What to do:**

Missing branches in `evict.rs` are expected in:

1. `evict_tip` — the "leaf is an only-child, causing parent to also become
   eligible for eviction" cascade.
2. `evict_tip` — the "leaf node has a depth gate exactly at threshold" boundary.
3. Any helper extracted in Phase 6 that was not tested inline.

Write tests that:
- Construct a `GvGraph` via `observe()` calls to reach the desired state.
- Call the eviction-triggering path directly or by making observations that
  drive budget eviction.
- Assert that the tree is in the expected post-eviction shape using the
  `is_leaf`, `has_children`, `child_count()` helpers from Phase 7.

**Acceptance:** `evict.rs` line coverage ≥ 90 %; `cargo test` passes.

---

### P9.4 — Add unit tests for `rebalance.rs`

**Status:** `[ ]` not started

**Depends on:** P9.1, P3.x (Phase 3 decomposition)

**Current coverage:** 78.65%.

**What to do:**

Missing branches in `rebalance.rs` are expected in:

1. `escalate_after_promote` — the "depth overflow" guard branch.
2. `resolve` — the "no-op" early return when headroom is already sufficient.
3. `rebalance` — the edge case when no nodes require headroom adjustment.

For each:
1. Identify the minimum observation sequence that produces the state triggering
   the branch.
2. Assert the tree depth and headroom after the operation.

**Acceptance:** `rebalance.rs` line coverage ≥ 90 %; `cargo test` passes.

---

### P9.5 — Add unit tests for `diagnostics/invariants.rs` and `diagnostic.rs`

**Status:** `[ ]` not started

**Depends on:** P9.1, P5.x (Phase 5 diagnostic decomposition)

**Current coverage:** `invariants.rs` 76.53%, `diagnostic.rs` 84.87%.

**What to do:**

The diagnostic modules validate codebase invariants; they are exercised when
invariants are violated.  Most tests exercise the "no violation found" path,
leaving the "violation detected and reported" paths uncovered.

To raise coverage:
1. Write tests that deliberately construct an invalid `GvGraph` state (bypass
   the public API using `#[cfg(test)]` helpers or direct struct construction if
   necessary) and assert that the invariant check returns an error.
2. For `diagnose_missed_violation` (CC=22), exercise every diagnostic message
   variant by constructing the corresponding violation scenario.
3. Use `#[cfg(test)]` builder helpers to avoid test boilerplate.

**Acceptance:** `invariants.rs` ≥ 90 %, `diagnostic.rs` ≥ 90 %; `cargo test` passes.

---

### P9.6 — Add unit tests for `query.rs`

**Status:** `[ ]` not started

**Depends on:** P9.1, P4.x (Phase 4 query decomposition)

**Current coverage:** 84.22%.

**What to do:**

`decompose_basis` (CC=20, Cog=40) is almost certainly the source of the missing
coverage.  Identify the uncovered branches:

1. The "no plateau basis found for this coordinate" early-return.
2. The "basis has a single element" degenerate path.
3. Any error path in `query.rs` that fires on empty-graph queries.

Write tests for each branch.  Tests should construct a graph with a controlled
plateau structure (using the helpers in `plateau_basis.rs`) and assert the
query result or the error variant.

**Acceptance:** `query.rs` line coverage ≥ 90 %; `cargo test` passes.

---

### P9.7 — Add unit tests for `budget.rs` and `contour_range.rs`

**Status:** `[ ]` not started

**Depends on:** P9.1

**Current coverage:** `budget.rs` 87.97%, `contour_range.rs` 87.50%.

**What to do:**

These files are close to the 90% target.  The gap is small:

- `budget.rs`: the uncovered line(s) are most likely in the `evict_candidates`
  error-handling or early-exit branches.  For example, "budget is empty on
  entry" or "no node is below the eviction threshold".
- `contour_range.rs`: the uncovered range is likely in an edge case of
  `ContourRange::merge` or `subtract` when one side is empty.

Write targeted tests for each uncovered branch identified from the coverage
report.

**Acceptance:** Both files ≥ 90 % line coverage; `cargo test` passes.

---

### P9.8 — Verify coverage for helpers added in Phases 1–8

**Status:** `[ ]` not started

**Depends on:** P9.7

**What to do:**

Run a final coverage report.  For each helper function added in Phases 1–8
that is not already covered by an existing test or a test added in P9.2–P9.7:

1. Identify the function in the coverage HTML (uncovered lines are shown in red).
2. Add a minimal unit test in the same file's `#[cfg(test)]` block.

The goal is not to achieve 100 % coverage everywhere — it is to ensure that
every *new* function introduced by the refactor has at least one test proving it
works correctly.

**Acceptance:** All helper functions from Phases 1–8 appear in at least one
test; overall codebase line coverage ≥ 90 %; `cargo test` passes.

---

### P9.9 — Update coverage baseline

**Status:** `[ ]` not started

**Depends on:** P9.8

**What to do:**

1. Run `cargo llvm-cov --workspace --html --output-dir target/coverage`.
2. Update the table in `docs/clean-code-plan/baseline.md` (Test Coverage
   section) with the new per-file numbers.
3. Commit the updated baseline.

**Acceptance:** `baseline.md` reflects post-Phase-9 coverage numbers.
