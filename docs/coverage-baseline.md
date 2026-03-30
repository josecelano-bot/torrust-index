# Test Coverage Baseline

Generated with:

```bash
cargo llvm-cov --summary-only
```

Tool: `cargo-llvm-cov 0.8.5`  
Date: 2026-03-30  
Tests: 531 total (517 unit tests in `src/`, 10 integration tests in `tests/integration.rs`, 4 snapshot tests in `tests/snapshot_tests.rs`)

---

## Summary

| Metric    | Covered | Total | Coverage   |
| --------- | ------: | ----: | ---------- |
| Regions   | 15284   | 17271 | **88.50%** |
| Functions | 1063    |  1138 | **93.41%** |
| Lines     | 9255    | 10427 | **88.76%** |

Compared with the previous baseline (2026-03-25):

- Regions: +1.48 pp
- Lines: +1.80 pp
- Functions: -3.26 pp

> Note: function coverage is not directly comparable to the prior snapshot because modules were split and additional helper/debug surfaces were introduced.

---

## Per-file Coverage (Selected)

| File                                | Lines | Missed | Line Cover | Functions | Missed Fn | Fn Cover |
| ----------------------------------- | ----: | -----: | ---------: | --------: | --------: | -------: |
| `graph/algorithm/budget.rs`         |   209 |     17 |     91.87% |        19 |         1 |   94.74% |
| `graph/algorithm/decay.rs`          |   268 |      3 |     98.88% |        25 |         0 |  100.00% |
| `graph/algorithm/evict.rs`          |   456 |     37 |     91.89% |        19 |         0 |  100.00% |
| `graph/algorithm/extract.rs`        |   185 |      5 |     97.30% |        17 |         0 |  100.00% |
| `graph/algorithm/observe.rs`        |   151 |     12 |     92.05% |        10 |         0 |  100.00% |
| `graph/algorithm/plateau/mod.rs`    |   236 |      1 |     99.58% |        25 |         0 |  100.00% |
| `graph/algorithm/promote.rs`        |   235 |     64 |     72.77% |         6 |         1 |   83.33% |
| `graph/algorithm/query.rs`          |   235 |      0 |    100.00% |        34 |         0 |  100.00% |
| `graph/algorithm/rebalance.rs`      |   401 |     36 |     91.02% |        31 |         0 |  100.00% |
| `graph/algorithm/split.rs`          |   140 |      0 |    100.00% |        12 |         0 |  100.00% |
| `diagnostics/invariants.rs`         |   646 |     62 |     90.40% |        60 |         0 |  100.00% |
| `diagnostics/plateau_invariants.rs` |   500 |    126 |     74.80% |        25 |         2 |   92.00% |
| `graph/gv_graph.rs`                 |   290 |     28 |     90.34% |        47 |         6 |   87.23% |
| `tree/vtree/mod.rs`                 |   344 |     15 |     95.64% |        39 |         3 |   92.31% |
| `tree/vtree/vnode.rs`               |   347 |      3 |     99.14% |        62 |         0 |  100.00% |

---

## Hotspots (Lowest Line Coverage)

| File                                                  | Line Cover | Notes |
| ----------------------------------------------------- | ---------: | ----- |
| `diagnostics/dot.rs`                                  |      0.00% | Dot rendering helpers currently not covered by tests |
| `graph/algorithm/plateau/noop_tracker.rs`             |      0.00% | No-op tracker path not selected in current test configuration |
| `traits/plateau_tracking.rs`                          |      0.00% | Trait-only surface without direct exercising test harness |
| `diagnostics/dump.rs`                                 |      9.66% | Diagnostic dump paths are mostly untested |
| `graph/algorithm/plateau/debug_api.rs`                |     23.08% | Debug-only API surface |
| `graph/algorithm/plateau/dynamic_tracker/debug_diff.rs` |   58.06% | Mostly diagnostic/debug diff branches |
| `graph/algorithm/query/get.rs`                        |     67.39% | Additional corner-case query paths still uncovered |
| `diagnostics/diagnostic/diagnose.rs`                  |     68.18% | Formatting/reporting permutations partially covered |
| `diagnostics/diagnostic/logging.rs`                   |     70.27% | Logging sink paths depend on runtime subscriber setup |
| `graph/algorithm/promote.rs`                          |     72.77% | Promotion edge cases remain the main algorithmic gap |

---

## Notes

- The previous eviction-related blocker is no longer reflected as a broad coverage bottleneck; `evict.rs`, `rebalance.rs`, `query.rs`, and `plateau/mod.rs` are now all above 90% line coverage (with `query.rs` at 100%).
- Remaining aggregate gaps are concentrated in diagnostics/debug paths and a few promotion/query helper branches.

---

## Regenerate

Summary table:

```bash
cargo llvm-cov --summary-only
```

HTML output (browsable, highlights uncovered lines):

```bash
cargo llvm-cov --html --output-dir docs/coverage-html
```

> Note: keep `docs/coverage-html/` in `.gitignore` because the HTML report is generated and large.
