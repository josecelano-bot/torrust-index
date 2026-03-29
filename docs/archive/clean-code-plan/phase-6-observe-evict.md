# Phase 6 — Observe / Evict / Budget Decomposition

## Goal

Reduce the complexity of the three tightly-coupled algorithm modules
(`observe.rs`, `evict.rs`, `budget.rs`) by decomposing their long functions
and making the data flow between phases explicit.

## Files targeted

- `src/graph/algorithm/observe.rs`
- `src/graph/algorithm/evict.rs`
- `src/graph/algorithm/budget.rs`

## Complexity baseline (2026-03-28)

### File summary

| CC  | Cognitive | SLOC | File                               |
| --- | --------- | ---- | ---------------------------------- |
|  41 |  36       |  398 | `graph/algorithm/evict.rs`         |
|  34 |  36       |  268 | `graph/algorithm/budget.rs`        |
|  30 |  25       |  229 | `graph/algorithm/observe.rs`       |

### Function hotspots

| CC  | Cognitive | SLOC | Function            | File                          |
| --- | --------- | ---- | ------------------- | ----------------------------- |
|  17 |  23       |  110 | `observe`           | `graph/algorithm/observe.rs`  |
|  16 |  28       |   83 | `evict_candidates`  | `graph/algorithm/budget.rs`   |
|  12 |  13       |  181 | `evict_tip`         | `graph/algorithm/evict.rs`    |

## Target (post-Phase 6)

| Metric                | Before | Target |
| --------------------- | -----: | -----: |
| Max function CC       |     17 |  ≤ 8   |
| Max function Cog      |     28 |  ≤ 15  |
| `evict_tip` SLOC      |    181 |  ≤ 50  |

---

## Background

These three modules implement the three main write operations on a `GvGraph`:

- **`observe`** — record a new observation at a coordinate, allocate a G-node
  if needed, split if needed, rebalance.
- **`evict_tip`** — remove a terminal G-node that has fallen below the eviction
  threshold, reattach its parent.
- **`evict_candidates`** / `check_evictions`** — scan the budget, identify
  nodes eligible for eviction, drive eviction in batches.

All three functions are "pipeline" functions: they execute a fixed sequence of
named steps, but those steps are not extracted into named helpers.  Reading
`observe` requires mentally tracking which step you are at among: `allocate →
insert into G-tree → insert into V-tree → propagate → split-if-needed →
rebalance`.

---

## Tasks

### P6.1 — Annotate pipeline phases in `observe`

**Status:** `[x]` done (phases were already annotated)

**What to do:**

1. Read `observe` in `observe.rs` top-to-bottom.
2. Identify each logical phase (allocation, G-tree insertion, V-tree insertion,
   value propagation, split guard, rebalance trigger).
3. Add a `// ── Phase N: <name> ──` banner comment above each phase (matching
   the style already used in `escalate_after_promote`).
4. Do not change any logic.

**Acceptance:** Annotations committed; `cargo test` passes.

---

### P6.2 — Extract `observe` phases into helpers (one per commit)

**Status:** `[ ]` not started

**Depends on:** P6.1

**What to do:**

Extract each annotated phase into a private `fn` on `GvGraph`:

| Helper fn name                    | Responsibility                                              |
| --------------------------------- | ----------------------------------------------------------- |
| `observe_allocate_or_find`        | Returns the `GNodeId` to update (allocate if new coord)    |
| `observe_insert_gtree`            | Inserts the node into the G-tree at the correct position   |
| `observe_insert_vtree`            | Attaches/updates the corresponding V-node                  |
| `observe_propagate_value`         | Walks the V-tree up, accumulating the new value            |
| `observe_split_if_needed`         | Triggers a split when the G-node exceeds its depth gate    |
| `observe_rebalance`               | Triggers the V-tree rebalance pass                         |

Each extraction is one commit.  `observe` body target: ≤ 20 SLOC.

**Acceptance:** `observe` CC ≤ 6, Cog ≤ 10; `cargo test` passes.

---

### P6.3 — Decompose `evict_tip`

**Status:** `[ ]` not started

**Depends on:** P6.2

**Baseline:** `evict_tip`, CC=12, SLOC=181.

The high SLOC count (181 lines for CC=12) signals a long body with many
sequential steps but limited branching.  The function:

1. Classifies the leaf removal (`classify_leaf_removal`).
2. Pushes eviction violations.
3. Detaches the node from the G-tree.
4. Patches the parent/grandparent pointers.
5. Notifies the plateau tracker.
6. Triggers rebalance.

**What to do:**

Extract steps 3–5 into named private helpers on `GvGraph` that capture what
they do rather than how:

| Helper fn name                    | Responsibility                                          |
| --------------------------------- | ------------------------------------------------------- |
| `evict_detach_from_gtree`         | Removes the evicted node from the G-tree structure      |
| `evict_patch_parent_pointers`     | Fixes parent/child pointers after detachment            |
| `evict_notify_plateau`            | Calls `plateau_tracker.on_evict(...)` with correct args |

**Acceptance:** `evict_tip` SLOC ≤ 50; `cargo test` passes.

---

### P6.4 — Simplify `evict_candidates` selection logic

**Status:** `[x]` done

**Depends on:** P6.3

**Baseline:** `evict_candidates`, CC=16, Cog=28, SLOC=83.

**What to do:**

`evict_candidates` scans the arena to find nodes eligible for eviction and
drives the eviction loop.  The high cognitive score comes from the nested
guard conditions inside the loop body.  Flatten by:

1. Extracting `is_eligible_for_eviction(node: &GNode<C,V>, config: &Config<V>) -> bool`
   — a pure predicate containing all the guard conditions.
2. Extracting `drive_eviction_batch(eligible: Vec<GNodeId>) -> u32` — takes the
   candidate list and drives `evict_tip` calls without the selection logic.

After extraction the loop body in `evict_candidates` is just:

```rust
if is_eligible_for_eviction(node, &self.config) {
    candidates.push(id);
}
```

**Acceptance:** `evict_candidates` Cog ≤ 12; `is_eligible_for_eviction` has a
unit test verifying both eligible and ineligible node types; `cargo test` passes.

---

### P6.5 — Add unit tests for observe and evict helpers

**Status:** `[ ]` not started

**Depends on:** P6.4

**What to do:**

Add focused unit tests for:

- `observe_allocate_or_find`: fresh graph, same coordinate observed twice →
  returns the same `GNodeId` on both calls.
- `is_eligible_for_eviction`: test with a node that is below the budget
  threshold (eligible = true) and a node that is above (eligible = false).
- `evict_detach_from_gtree`: after detachment, the arena no longer contains the
  node and the parent's child pointer is cleared.

**Acceptance:** ≥ 3 new unit tests; `cargo test` passes.

---

## Commit message template

```
refactor(observe|evict|budget): <what>

<why — reference cognitive complexity numbers and the specific helper extracted>

Part of docs/clean-code-plan/phase-6-observe-evict.md task P6.x.
```
