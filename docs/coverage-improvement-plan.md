# Coverage Improvement Plan

Date: 2026-03-30
Source baseline: [docs/coverage-baseline.md](docs/coverage-baseline.md)
Full report: [docs/coverage-report-full-2026-03-30.txt](docs/coverage-report-full-2026-03-30.txt)

## Goal

Increase line coverage from 88.76% with progress tracked per file.

## Status Legend

- NS: not started
- IP: in progress
- DONE: reached target for this file (or explicitly accepted)
- BLOCKED: blocked by design/runtime constraint

## Per-file Tracker

Update `Status`, `Current`, and `Notes` after each coverage run.

| File | Baseline | Phase | Status | Current | Notes |
| ---- | -------: | ----: | :----: | ------: | ----- |
| `diagnostics/dot.rs` | 0.00% | 1 | NS | - | |
| `graph/algorithm/plateau/noop_tracker.rs` | 0.00% | 1 | NS | - | |
| `traits/plateau_tracking.rs` | 0.00% | 1 | NS | - | |
| `diagnostics/dump.rs` | 9.66% | 1 | NS | - | |
| `graph/algorithm/plateau/debug_api.rs` | 23.08% | 1 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/debug_diff.rs` | 58.06% | 1 | NS | - | |
| `graph/algorithm/query/get.rs` | 67.39% | 1 | NS | - | |
| `diagnostics/diagnostic/diagnose.rs` | 68.18% | 1 | NS | - | |
| `diagnostics/diagnostic/logging.rs` | 70.27% | 1 | NS | - | |
| `graph/algorithm/promote.rs` | 72.77% | 1 | NS | - | |
| `diagnostics/diagnostic.rs` | 74.19% | 1 | NS | - | |
| `diagnostics/plateau_invariants.rs` | 74.80% | 1 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs` | 80.21% | 2 | NS | - | |
| `graph/algorithm/rebalance/resolve.rs` | 85.00% | 2 | NS | - | |
| `diagnostics/plateau_audit.rs` | 85.90% | 2 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs` | 86.69% | 2 | NS | - | |
| `graph/algorithm/query/contour.rs` | 88.39% | 2 | NS | - | |
| `tree/gtree/gnode_tree.rs` | 89.51% | 2 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/split_bootstrap.rs` | 89.66% | 2 | NS | - | |
| `graph/algorithm/query/range_sum.rs` | 89.80% | 2 | NS | - | |
| `graph/gv_graph.rs` | 90.34% | 3 | NS | - | |
| `diagnostics/invariants.rs` | 90.40% | 3 | NS | - | |
| `graph/algorithm/plateau/update_wrappers.rs` | 90.57% | 3 | NS | - | |
| `graph/algorithm/violation_push.rs` | 90.71% | 3 | NS | - | |
| `graph/algorithm/rebalance.rs` | 91.02% | 3 | NS | - | |
| `graph/algorithm/budget.rs` | 91.87% | 3 | NS | - | |
| `graph/algorithm/evict.rs` | 91.89% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/legacy_promotes.rs` | 91.89% | 3 | NS | - | |
| `graph/algorithm/observe.rs` | 92.05% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/evict.rs` | 92.59% | 3 | NS | - | |
| `graph/algorithm/rebalance/violation_scan.rs` | 92.86% | 3 | NS | - | |
| `diagnostics/display.rs` | 93.06% | 3 | NS | - | |
| `tree/vtree/vnode_tree.rs` | 93.28% | 3 | NS | - | |
| `spatial/contour_range.rs` | 94.04% | 3 | NS | - | |
| `graph/algorithm/split/helpers.rs` | 94.34% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/repair.rs` | 94.74% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/observe.rs` | 95.00% | 3 | NS | - | |
| `tree/vtree/mod.rs` | 95.64% | 3 | NS | - | |
| `diagnostics/invariants/reporting.rs` | 95.74% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/debug_sums.rs` | 95.83% | 3 | NS | - | |
| `graph/algorithm/plateau/read_api.rs` | 96.47% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/normalize.rs` | 97.06% | 3 | NS | - | |
| `graph/algorithm/extract.rs` | 97.30% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/split_catalytic.rs` | 97.35% | 3 | NS | - | |
| `graph/algorithm/sample.rs` | 97.67% | 3 | NS | - | |
| `spatial/pewei/core.rs` | 97.91% | 3 | NS | - | |
| `spatial/plateau_basis.rs` | 98.32% | 3 | NS | - | |
| `tree/gtree/gnode.rs` | 98.37% | 3 | NS | - | |
| `graph/algorithm/plateau/dynamic_tracker/core_helpers/place.rs` | 98.69% | 3 | NS | - | |
| `graph/algorithm/decay.rs` | 98.88% | 3 | NS | - | |
| `tree/vtree/vnode.rs` | 99.14% | 3 | NS | - | |
| `graph/algorithm/plateau/mod.rs` | 99.58% | 3 | NS | - | |

## Phase Targets

1. Phase 1 complete when all Phase 1 files are `DONE` or `BLOCKED`.
2. Phase 2 complete when all Phase 2 files are `DONE` or `BLOCKED`.
3. Phase 3 complete when all Phase 3 files are `DONE` or `BLOCKED`.

## Runbook

1. Pick one file from the highest-priority phase still open.
2. Add or update tests for uncovered branches in that file.
3. Run `cargo test`.
4. Run `cargo llvm-cov --summary-only`.
5. Update that file row (`Status`, `Current`, `Notes`).

## Coverage Milestones

1. After Phase 1: line coverage at or above 91%.
2. After Phase 2: line coverage at or above 93%.
3. After Phase 3: line coverage at or above 95%.

## Change Log

| Date | Files Updated | Result |
| ---- | ------------- | ------ |
| 2026-03-30 | Tracker initialized | All files set to `NS` |
| 2026-03-30 | diagnostics, query, promote, rebalance/resolve, dynamic tracker helpers, invariants, vtree/vnode helpers | Reached line coverage milestones: Phase 1 >= 91%, Phase 2 >= 93%, Phase 3 >= 95% (current total: 95.01%) |
