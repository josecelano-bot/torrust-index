# Phase 4 - Stabilization and Proof

## Delivery Intent

Prove improvements with refreshed metrics and document what complexity remains essential.

## Deliverables

1. Fresh complexity measurement snapshot.
2. Before/after hotspot comparison.
3. Residual complexity map (essential vs accidental).
4. Follow-up backlog for unresolved accidental complexity.

## Step Checklist

### P4.1 Metrics refresh

- [x] Recompute complexity metrics using established workflow
- [x] Compare top hotspots before and after
- [x] Update complexity analysis documentation

### P4.2 Residual complexity map

- [x] Classify remaining complex functions as essential vs accidental
- [x] Add explicit follow-up tasks for accidental leftovers
- [x] Record final decision notes

## Acceptance Criteria

- [x] No function above CC 20
- [x] Remaining cognitive outliers have justification or follow-up
- [x] Full test suite passes

## Test Checklist (per step)

- [x] Gate A: `cargo check --all-features`
- [x] Gate B: targeted tests for touched area
- [x] Gate C: `cargo test --all-features`

## Results Snapshot (2026-03-29)

### Complexity Refresh

- Snapshot path: `metrics-output/recheck-2026-03-29-stabilization/`
- Tool: `rust-code-analysis-cli 0.0.25`
- Scope: all `.rs` files under `src/`

| Metric | Previous (recheck) | Current (stabilization) | Delta |
| --- | ---: | ---: | ---: |
| Analyzed files | 98 | 102 | +4 |
| Total functions/closures | 1101 | 1133 | +32 |
| Total CC (sum) | 2359 | 2383 | +24 |
| Total Cognitive (sum) | 1494 | 1460 | -34 |
| Total SLOC | 17838 | 18131 | +293 |
| Functions with CC > 10 | 29 | 27 | -2 |
| Functions with CC > 20 | 0 | 0 | 0 |
| Max function CC | 20 | 17 | -3 |

### Hotspot Comparison (Function Level)

- Previous top CC function: `decompose_basis` (CC=20, Cog=40)
- Current top CC functions: `build_plateaus` (CC=17, Cog=39), `observe` (CC=17, Cog=23)
- Snapshot cognitive outlier: `evict_ancestor_key` (CC=16, Cog=49); helper extraction pass completed after snapshot, re-measure pending.

### Benchmark Refresh

- Command: `cargo bench --bench depth`
- Current results:
  - `observe/steady_state`: 433.09 ns - 442.52 ns
  - `observe/split_heavy`: 140.86 ns - 141.98 ns
- Reference baseline (archived): `docs/archive/refactoring/phase-6.md`
  - `observe/steady_state`: ~285 ns
  - `observe/split_heavy`: ~97 ns

## Residual Complexity Map

### Essential (domain-driven / algorithmic)

1. `build_plateaus` (`src/graph/algorithm/plateau/read_api.rs`)
2. `observe` (`src/graph/algorithm/observe.rs`)
3. `normalize_impl` (`src/graph/algorithm/plateau/dynamic_tracker/normalize.rs`)
4. `rebalance` (`src/graph/algorithm/rebalance.rs`)

Reasoning: these coordinate multi-step graph transitions with correctness constraints across structural/value trees and plateau invariants.

### Accidental (candidate for follow-up extraction)

1. `evict_ancestor_key` (`src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs`, Cog=49)
2. `collect_normalize_elements` (`src/graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs`, Cog=32)
3. `on_catalytic_split_impl` (`src/graph/algorithm/plateau/dynamic_tracker/split_catalytic.rs`, Cog=31)

Reasoning: complexity is driven by branching/nesting and mixed responsibilities that can be split into smaller phase helpers without changing the algorithm.

## Follow-up Backlog

1. [x] Extract `evict_ancestor_key` into branch-specific phase helpers (selection, displacement, merge, reinsert).
2. [x] Split `collect_normalize_elements` into traversal/selection and merge-policy components.
3. Isolate `on_catalytic_split_impl` post-split normalization from mirror/debug responsibilities.

## Progress Log

### Step Template

- Step:
- Status: [ ]
- Files:
- Commit:
- Tests:
  - [ ] Gate A
  - [ ] Gate B
  - [ ] Gate C
- Notes:
- Follow-up:

### Completed Steps

- Step: P4.1 Metrics refresh
- Status: [x]
- Files:
  - `docs/complexity-analysis.md`
  - `docs/refactoring-plans/PROGRESS.md`
  - `docs/refactoring-plans/accidental-complexity-plan/phase-4-stabilization.md`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes:
  - Fresh complexity snapshot generated at `metrics-output/recheck-2026-03-29-stabilization/`.
  - No functions above CC 20; max function CC now 17.
- Follow-up:
  - Keep snapshot path stable for future deltas.

- Step: P4.2 Residual complexity map
- Status: [x]
- Files:
  - `docs/refactoring-plans/accidental-complexity-plan/phase-4-stabilization.md`
  - `docs/refactoring-plans/PROGRESS.md`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes:
  - Residual complex functions classified as essential vs accidental.
  - Follow-up backlog captured for accidental hotspots.
- Follow-up:
  - Execute backlog in a post-refactor hardening pass.

- Step: P4 follow-up backlog item 1 (`evict_ancestor_key` extraction)
- Status: [x]
- Files:
  - `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs`
  - `docs/refactoring-plans/accidental-complexity-plan/phase-4-stabilization.md`
  - `docs/refactoring-plans/PROGRESS.md`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes:
  - Split `evict_ancestor_key` into helper phases (`covering_ancestor_key`, `displace_semi_internal_survivor`, `displace_path_siblings`) while preserving behavior and ordering.
- Follow-up:
  - Recompute complexity snapshot to capture post-extraction cognitive delta.

- Step: P4 follow-up backlog item 2 (`collect_normalize_elements` extraction)
- Status: [x]
- Files:
  - `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs`
  - `docs/refactoring-plans/accidental-complexity-plan/phase-4-stabilization.md`
  - `docs/refactoring-plans/PROGRESS.md`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes:
  - Split normalize-element gathering into focused traversal helpers (`basis_ids_snapshot`, `collect_from_basis_root`, `process_normalize_node`, `push_normalize_element`).
  - Preserved normalize behavior while reducing mixed traversal/collection branching inside the public helper.
- Follow-up:
  - Recompute complexity snapshot to capture post-extraction cognitive delta.
