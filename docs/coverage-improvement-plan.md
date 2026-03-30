# Coverage Improvement Plan

Date: 2026-03-30
Source baseline: [docs/coverage-baseline.md](docs/coverage-baseline.md)
Full report: [docs/coverage-report-full-2026-03-30.txt](docs/coverage-report-full-2026-03-30.txt)

## Goal

Increase line coverage from 88.76% by prioritizing files with the largest uncovered line counts and highest runtime risk.

## Prioritization Rules

1. Lowest line coverage first.
2. Runtime algorithm modules before debug-only utilities.
3. High uncovered-line count before tiny files.
4. Add deterministic unit tests first, then scenario/integration tests.

## Phase 1: Critical Gaps (under 75% lines)

1. diagnostics/dot.rs (0.00%)
2. graph/algorithm/plateau/noop_tracker.rs (0.00%)
3. traits/plateau_tracking.rs (0.00%)
4. diagnostics/dump.rs (9.66%)
5. graph/algorithm/plateau/debug_api.rs (23.08%)
6. graph/algorithm/plateau/dynamic_tracker/debug_diff.rs (58.06%)
7. graph/algorithm/query/get.rs (67.39%)
8. diagnostics/diagnostic/diagnose.rs (68.18%)
9. diagnostics/diagnostic/logging.rs (70.27%)
10. graph/algorithm/promote.rs (72.77%)
11. diagnostics/diagnostic.rs (74.19%)
12. diagnostics/plateau_invariants.rs (74.80%)

Planned test work:
- Add direct unit tests for formatter and dump helpers with stable snapshots.
- Add config-driven tests that force noop tracker path selection.
- Add trait contract tests for plateau tracking trait surface.
- Add promote edge-case tests for parent/ancestor permutations and boundary splits.
- Add query get tests for endpoint clamping, uncovered intervals, and semi-internal node paths.
- Add logging tests with a test subscriber to execute lazy formatting branches.

## Phase 2: Structural Gaps (75% to 90% lines)

1. graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs (80.21%)
2. graph/algorithm/rebalance/resolve.rs (85.00%)
3. diagnostics/plateau_audit.rs (85.90%)
4. graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs (86.69%)
5. graph/algorithm/query/contour.rs (88.39%)
6. tree/gtree/gnode_tree.rs (89.51%)
7. graph/algorithm/plateau/dynamic_tracker/split_bootstrap.rs (89.66%)
8. graph/algorithm/query/range_sum.rs (89.80%)

Planned test work:
- Build small synthetic trees to hit consolidate and fixup rare paths.
- Add rebalance resolve tests for all exit conditions and fallback behavior.
- Add contour and range_sum interval matrix tests for open and closed boundary variants.
- Add gnode_tree shape-transition tests for removal, reparenting, and leaf collapse.

## Phase 3: Completion Sweep (90% to under 100% lines)

Targets include:
- graph/gv_graph.rs
- diagnostics/invariants.rs
- graph/algorithm/plateau/update_wrappers.rs
- graph/algorithm/violation_push.rs
- graph/algorithm/rebalance.rs
- graph/algorithm/budget.rs
- graph/algorithm/evict.rs
- graph/algorithm/observe.rs
- diagnostics/display.rs
- tree/vtree/vnode_tree.rs
- spatial/contour_range.rs
- graph/algorithm/split/helpers.rs
- graph/algorithm/plateau/dynamic_tracker/repair.rs
- graph/algorithm/plateau/dynamic_tracker/observe.rs
- tree/vtree/mod.rs
- diagnostics/invariants/reporting.rs
- graph/algorithm/plateau/dynamic_tracker/debug_sums.rs
- graph/algorithm/plateau/read_api.rs
- graph/algorithm/plateau/dynamic_tracker/normalize.rs
- graph/algorithm/extract.rs
- graph/algorithm/plateau/dynamic_tracker/split_catalytic.rs
- graph/algorithm/sample.rs
- spatial/pewei/core.rs
- spatial/plateau_basis.rs
- tree/gtree/gnode.rs
- graph/algorithm/plateau/dynamic_tracker/core_helpers/place.rs
- graph/algorithm/decay.rs
- tree/vtree/vnode.rs
- graph/algorithm/plateau/mod.rs

Planned test work:
- Add branch-focused micro-tests for uncovered guards and error returns.
- Use parameterized tests for boundary and empty-input cases.
- Prefer test names mapped to specific branch intent.

## Iteration Workflow

1. Implement tests for one phase only.
2. Run: cargo test
3. Run: cargo llvm-cov --summary-only
4. Record before and after coverage deltas in this document.
5. Repeat for next phase.

## Success Targets

1. After Phase 1: line coverage at or above 91%.
2. After Phase 2: line coverage at or above 93%.
3. After Phase 3: line coverage at or above 95%.

## Notes

- Keep debug-only files in scope, but prioritize runtime logic first.
- If a path is intentionally unreachable in production, document the rationale next to the test gap.
