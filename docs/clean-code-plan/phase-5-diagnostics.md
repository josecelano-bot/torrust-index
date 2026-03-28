# Phase 5 — Diagnostics Simplification

## Goal

Reduce the cognitive load of the diagnostics subsystem by decomposing long
invariant-checker functions, eliminating duplicated traversal patterns, and
giving each check a clear, single responsibility.

## Files targeted

- `src/diagnostics/invariants.rs`
- `src/diagnostics/plateau_invariants.rs`
- `src/diagnostics/diagnostic.rs`
- `src/diagnostics/dump.rs`

## Complexity baseline (2026-03-28)

### File summary

| CC  | Cognitive | SLOC | File                                      |
| --- | --------- | ---- | ----------------------------------------- |
| 128 |       185 |  622 | `diagnostics/plateau_invariants.rs`       |
| 115 |       171 |  629 | `diagnostics/invariants.rs`              |
|  52 |        42 |  349 | `diagnostics/diagnostic.rs`              |
|  52 |        30 |  249 | `diagnostics/dump.rs`                    |

### Function hotspots

| CC  | Cognitive | SLOC | Function                                     | File                           |
| --- | --------- | ---- | -------------------------------------------- | ------------------------------ |
|  24 |  16       |  115 | `dump_plateaus`                              | `diagnostics/dump.rs`          |
|  22 |  21       |  139 | `diagnose_missed_violation`                  | `diagnostics/diagnostic.rs`   |
|  13 |  19       |   44 | `contour_steps`                              | `diagnostics/plateau_invariants.rs` |
|  13 |  13       |   57 | `check_p_i1_i_keys_are_contour_steps`        | `diagnostics/plateau_invariants.rs` |
|  12 |  28       |   57 | `audit_plateau_consistency`                  | `diagnostics/plateau_audit.rs` |
|  12 |  22       |   69 | `check_p_i1_ii_tile_contiguity`              | `diagnostics/plateau_invariants.rs` |
|  12 |  25       |   70 | `check_p_i1_iii_run_contains_tile`           | `diagnostics/plateau_invariants.rs` |
|  12 |  19       |   42 | `check_p_i5_thatch_depth`                    | `diagnostics/plateau_invariants.rs` |
|  11 |  13       |   54 | `check_p_i4_thatch_one_hop`                  | `diagnostics/plateau_invariants.rs` |

## Target (post-Phase 5)

| Metric                     | Before | Target |
| -------------------------- | -----: | -----: |
| `plateau_invariants.rs` CC |    128 |  ≤ 70  |
| `invariants.rs` CC         |    115 |  ≤ 70  |
| Max function CC            |     24 |  ≤ 10  |
| Max function Cog           |     28 |  ≤ 15  |

---

## Background

The diagnostics subsystem validates codebase invariants during development and
in debug builds.  It is production-inert but architecturally important: if a
check is hard to read, a failing assertion gives no clear signal about which
invariant was violated or why.

The high cognitive scores come from two patterns:

1. **Nested traversal with early-error returns** — functions like
   `check_p_i1_ii_tile_contiguity` walk a tree node-by-node, carrying multiple
   accumulator variables and short-circuiting on the first inconsistency.  The
   nesting makes the traversal shape invisible.

2. **Mixed traversal + validation** — functions like `diagnose_missed_violation`
   both walk the structure and compute the diagnostic message in the same body,
   making it hard to test the diagnostics in isolation from the traversal.

---

## Tasks

### P5.1 — Extract `dump_plateaus` into a traversal + renderer split

**Status:** `[x]` done

**File:** `diagnostics/dump.rs`

**What to do:**

`dump_plateaus` (CC=24, SLOC=115) both traverses the plateau data structure
and formats each entry.  Split it:

1. `collect_plateau_dump_rows` — traverses the plateau map, returns a
   `Vec<PlateauRow>` (a plain data struct, no formatting).
2. `render_plateau_rows` — takes `&[PlateauRow]` and writes the formatted output.

`dump_plateaus` becomes a two-line orchestrator.

**Acceptance:** `dump_plateaus` CC ≤ 5; `cargo test` passes.

---

### P5.2 — Extract `diagnose_missed_violation` evidence builder

**Status:** `[x]` done

**Depends on:** P5.1

**File:** `diagnostics/diagnostic.rs`

**What to do:**

`diagnose_missed_violation` (CC=22, Cog=21, SLOC=139) collects evidence and
formats an error message in one monolithic body.  Split:

1. `collect_violation_evidence` — walks the V-tree from the reported node
   upward, returns a `ViolationEvidence` struct (ancestor chain, intensity values,
   uncle intensities).
2. `format_violation_message` — takes `&ViolationEvidence`, returns `String`.
3. `diagnose_missed_violation` becomes: call `collect_evidence`, call `format`,
   return.

**Acceptance:** `diagnose_missed_violation` CC ≤ 6; `cargo test` passes.

---

### P5.3 — Extract shared traversal helpers in `plateau_invariants.rs`

**Status:** `[ ]` not started

**Depends on:** P5.2

**File:** `diagnostics/plateau_invariants.rs`

**What to do:**

Multiple `check_p_i1_*` functions repeat the same pattern:

```rust
for (basis_key, plateau) in &plateau_map {
    for tile_id in &plateau.tiles {
        let tile = g_nodes.get(tile_id.index());
        // … specific check …
    }
}
```

Extract:

```rust
fn for_each_plateau_tile<F>(
    plateau_map: &PlateauBasisMap<C, V>,
    g_nodes:     &Arena<GNode<C, V>>,
    mut f:       F,
) where F: FnMut(&PlateauKey<C>, &Plateau<C, V>, GNodeId, &GNode<C, V>)
```

Then each `check_p_i1_*` function becomes a call to `for_each_plateau_tile`
with a closure containing only the specific check logic.

**Acceptance:** Duplicated traversal loop removed; `cargo test` passes; no
check function body > 25 SLOC.

---

### P5.4 — Reduce nesting in `contour_steps`

**Status:** `[x]` done

**Depends on:** P5.3

**File:** `diagnostics/plateau_invariants.rs`

**Baseline:** `contour_steps`, CC=13, Cog=19, SLOC=44.

**What to do:**

`contour_steps` builds a set of contour step coordinates by walking the G-tree.
The high cognitive score comes from the nested `if let Some(left) { if let
Some(right) { ... } }` pattern.  Flatten using:

1. `let Some(left) = node.left() else { continue; }` early-return style.
2. Extract the "is this a contour step?" predicate into `is_contour_step_node`.

**Acceptance:** `contour_steps` Cog ≤ 10; `cargo test` passes.

---

### P5.5 — Add targeted unit tests for diagnostic helpers

**Status:** `[ ]` not started

**Depends on:** P5.4

**What to do:**

The extracted `collect_plateau_dump_rows`, `collect_violation_evidence`, and
`is_contour_step_node` should each have at least one unit test.  Use the
existing `make_config()` / `fresh()` scaffolding pattern.

**Acceptance:** ≥ 1 unit test per extracted helper; `cargo test` passes.

---

## Commit message template

```
refactor(diagnostics): <what>

<why — reference cognitive complexity numbers and the specific helper extracted>

Part of docs/clean-code-plan/phase-5-diagnostics.md task P5.x.
```
