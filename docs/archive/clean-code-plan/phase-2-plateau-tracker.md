# Phase 2 — Plateau Tracker Decomposition

## Goal

Reduce `dynamic_tracker.rs` from the most complex file in the codebase (CC=214,
Cog=311, 1 417 lines) to a set of focused, individually-testable functions, each
with a clear and well-named purpose.

## File targeted

- `src/graph/algorithm/plateau/dynamic_tracker.rs`

## Complexity baseline (2026-03-28)

| CC  | Cognitive | SLOC | Function                        |
| --- | --------- | ---- | ------------------------------- |
|  41 | **95**    |  201 | `on_evict`                      |
|  26 |  46       |  147 | `normalize`                     |
|  17 |  11       |  128 | `place_basis_element`           |
|  15 |  17       |   85 | `fixup_plateau`                 |
|  12 |  17       |   48 | `repair_p_i4`                   |
|  12 |  16       |  109 | `debug_assert_mirror_consistency` |
|  11 | **31**    |   98 | `on_catalytic_split`            |

File aggregate: **CC=214, Cog=311**.

## Target (post-Phase 2)

| Metric            | Before | Target |
| ----------------- | -----: | -----: |
| File CC           |    214 |  ≤ 100 |
| File Cognitive    |    311 |  ≤ 150 |
| Max function CC   |     41 |  ≤ 10  |
| Max function Cog  |     95 |  ≤ 20  |

---

## Background

`dynamic_tracker.rs` is the result of Phase-7 extraction in the previous series:
the plateau subsystem was pulled out of `GvGraph` into a self-contained struct.
The extraction was correct and necessary, but the functions themselves were not
further decomposed — they were moved as-is.  The file is now a "big bag of
plateau algorithms" rather than a set of cohesive, named responsibilities.

The dominant hotspot is `on_evict` (CC=41, Cog=95): a 201-line function that
handles five distinct logical phases after a V-node leaf is removed.  These
phases are identifiable from the existing banner comments but are not separated
into helper functions.

---

## Tasks

### P2.1 — Read and annotate `on_evict`

**Status:** `[x]` done (phase-banner comments were already present)

---

### P2.2 — Extract `on_evict` phase helpers (one per commit)

**Status:** `[x]` done

Extracted:
- `evict_ancestor_key` — Phase 1 else-branch (ancestor walk + sibling displacement)
- `evacuate_adjacent_plateaus` — Phases 4+5 (right and left adjacency evacuation)

`on_evict` reduced from ~228 lines to ~80 lines.

---

### P2.3 — Extract `normalize` phase helpers

**Status:** `[x]` done

Extracted `collect_normalize_elements` — Phase 1 DFS collection loop.
`normalize` Phase 1 reduced from ~80 lines to 1 call.

---

### P2.4 — Extract `place_basis_element` helpers

**Status:** `[x]` done

Extracted:
- `find_adjacent_left_key` — left adjacency predicate
- `find_adjacent_right_key` — right adjacency predicate
- `place_merge_both` — (Some(lk), Some(rk)) arm
- `place_extend_left` — (Some(lk), None) arm
- `place_rekey_right` — (None, Some(rk)) arm
- `place_new_plateau` — (None, None) arm

`place_basis_element` reduced from 128 lines to 25 lines.

---

### P2.5 — Extract `on_catalytic_split` helpers

**Status:** `[ ]` not started — deferred; function complexity acceptable after P2.2–P2.4.

**What to do:**

1. Read `on_evict` from top to bottom.
2. Identify the logical phases (the existing banner comments should match).
3. For each phase, write a one-line description of:
   - What inputs it reads.
   - What state it mutates.
   - What the output is passed to the next phase.
4. Write these descriptions as comments directly above the phase banners (do not
   extract yet — this is a reading and annotation step).

**Acceptance:** Annotations committed; no behaviour change; `cargo test` passes.

---

### P2.2 — Extract `on_evict` phase helpers (one per commit)

**Status:** `[ ]` not started

**Depends on:** P2.1

**What to do:**

Extract each logical phase of `on_evict` into a private `fn` on
`DynamicPlateauTracker`.  Do one extraction per commit.  Suggested split:

| Helper fn name                          | Responsibility                                            |
| --------------------------------------- | --------------------------------------------------------- |
| `evict_remove_terminal_from_basis`      | Removes the evicted node from the plateau basis           |
| `evict_reparent_or_dissolve_plateau`    | Handles the structural plateau repair after removal       |
| `evict_check_parent_plateau_merge`      | Checks and merges the parent's plateau if it becomes stale|
| `evict_fixup_grandparent_plateau`       | The grandparent plateau fixup pass                        |
| `evict_rebuild_contiguous_run`          | Rebuilds contiguous-run invariants after eviction         |

Each helper should:
- Take only the parameters it needs (avoid threading `&mut self` through every
  helper if only a subset of fields is mutated).
- Have a `/// <purpose>` doc comment.
- Be exercisable by the existing test suite (verify with `cargo test` after each).

**Acceptance:** After all commits: `on_evict` body is ≤ 40 SLOC; CC ≤ 10; all
tests pass.

---

### P2.3 — Extract `normalize` phase helpers

**Status:** `[ ]` not started

**Depends on:** P2.2

**Baseline:** `normalize`, CC=26, Cog=46, SLOC=147.

**What to do:**

`normalize` rebuilds the plateau mirror after a mutating operation.  It has at
least three identifiable sub-responsibilities:

1. Collect the set of basis edges that need updating.
2. For each edge, recompute the plateau span.
3. Update the reverse index (`plateau_basis`).

Extract each into a helper:

| Helper fn name                      | Responsibility                                          |
| ----------------------------------- | ------------------------------------------------------- |
| `collect_stale_basis_edges`         | Returns the list of edges that need recomputation       |
| `recompute_plateau_for_edge`        | Rebuilds a single plateau given the current G-tree state|
| `commit_plateau_update`             | Writes the result back and updates the reverse index    |

**Acceptance:** `normalize` CC ≤ 8; all tests pass.

---

### P2.4 — Extract `place_basis_element` helpers

**Status:** `[ ]` not started

**Depends on:** P2.3

**Baseline:** `place_basis_element`, CC=17, SLOC=128.

**What to do:**

`place_basis_element` determines where a newly split node fits into the existing
plateau basis.  It has at least three cases:

1. The new node slots into an existing plateau without changing its boundaries.
2. The new node causes a plateau boundary to shift.
3. The new node creates a new independent plateau.

Extract these as a match on an enum (or three private helpers), so the top-level
function reads as a dispatcher:

```rust
fn place_basis_element(...) {
    match self.classify_basis_placement(...) {
        BasisPlacement::SlotIn     => self.slot_into_existing_plateau(...),
        BasisPlacement::ShiftBound => self.shift_plateau_boundary(...),
        BasisPlacement::NewPlateau => self.create_new_plateau(...),
    }
}
```

**Acceptance:** `place_basis_element` CC ≤ 6; each case ≤ 20 SLOC; all tests pass.

---

### P2.5 — Extract `on_catalytic_split` helpers

**Status:** `[ ]` not started

**Depends on:** P2.4

**Baseline:** `on_catalytic_split`, CC=11, Cog=31.

**What to do:**

`on_catalytic_split` reacts to a new node produced by a split during the
`observe` pipeline.  The high cognitive score (31 vs CC=11) indicates nested
conditionals rather than many branches — flatten where possible.

Concrete steps:
1. Extract the guard logic (early-return conditions) into a `split_is_trivial`
   predicate or by restructuring with early returns.
2. Extract the "update parent plateau" slice into a helper.
3. Extract the "insert child into basis" slice into a helper.

**Acceptance:** `on_catalytic_split` CC ≤ 6 and Cog ≤ 15; all tests pass.

---

### P2.6 — Add unit tests for extracted helpers

**Status:** `[ ]` not started

**Depends on:** P2.5

**What to do:**

The helpers extracted in P2.2–P2.5 should each have at least one unit test that
exercises them in isolation:

- Construct a minimal `DynamicPlateauTracker` and a stub `Arena<GNode>`.
- Call the helper directly.
- Assert the expected map mutation.

This is only feasible because Phase 1 (encapsulation) will have made the helper
inputs explicit rather than `&mut self`; adjust if Phase 1 is not yet complete.

**Acceptance:** ≥ 1 unit test per extracted helper; `cargo test` passes.

---

### P2.7 — Split `dynamic_tracker.rs` into sub-files if cohesion warrants it

**Status:** `[ ]` not started

**Depends on:** P2.6

**What to do:**

After extraction, assess whether the file can be split cleanly along
responsibility lines, for example:

- `plateau/tracker/evict.rs` — `on_evict` and its helpers
- `plateau/tracker/split.rs` — `on_catalytic_split`, `on_observe` helpers
- `plateau/tracker/normalise.rs` — `normalize` and helpers
- `plateau/tracker/mod.rs` — struct definition, `PlateauTracking` impl, `with_root`

Only split if the current file is still > 600 SLOC after P2.2–P2.6; forced
splits add navigation cost without benefit.

**Acceptance:** If split: each sub-file ≤ 300 SLOC; `cargo test` passes.

---

## Commit message template

```
refactor(plateau): <what>

<why — reference cognitive complexity numbers and the specific helper extracted>

Part of docs/clean-code-plan/phase-2-plateau-tracker.md task P2.x.
```
