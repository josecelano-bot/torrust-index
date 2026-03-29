# Complexity Baseline — 2026-03-28

Generated with `rust-code-analysis-cli 0.0.25` after the Q/R-series refactors.

```bash
~/.local/bin/rust-code-analysis-cli -m -p src/ -O json --pr -o metrics-output/
```

This is the **starting baseline** for the [clean-code refactor plan](index.md).

---

## Files by Aggregate Cyclomatic Complexity

> File-level CC is the sum of all function/closure CC values in that file.
> Cognitive = sum of cognitive complexity scores.

| CC  | Cognitive | SLOC | File                                             |
| --- | --------- | ---- | ------------------------------------------------ |
| 214 | 311       | 1417 | `src/graph/algorithm/plateau/dynamic_tracker.rs` |
| 128 | 185       |  622 | `src/diagnostics/plateau_invariants.rs`          |
| 115 | 171       |  629 | `src/diagnostics/invariants.rs`                  |
| 110 | 107       |  739 | `src/graph/algorithm/rebalance.rs`               |
|  90 |  68       |  628 | `src/graph/algorithm/query.rs`                   |
|  87 |  17       |  631 | `src/nodes/vnode.rs`                             |
|  81 |  53       |  596 | `src/tree/vtree.rs`                              |
|  75 |  12       |  476 | `src/nodes/gnode.rs`                             |
|  67 |  52       |  378 | `src/graph/algorithm/violation_push.rs`          |
|  65 |  49       |  402 | `src/graph/algorithm/plateau/mod.rs`             |
|  65 |   0       |  510 | `src/traits/coordinate.rs`                       |
|  63 |   7       |  585 | `src/graph/gv_graph.rs`                          |
|  62 |  59       |  432 | `src/graph/algorithm/decay.rs`                   |
|  59 |  15       |  781 | `src/spatial/pewei.rs`                           |
|  52 |  42       |  349 | `src/diagnostics/diagnostic.rs`                  |
|  52 |  30       |  249 | `src/diagnostics/dump.rs`                        |
|  51 |   7       |  415 | `src/arena.rs`                                   |
|  41 |  36       |  398 | `src/graph/algorithm/evict.rs`                   |
|  41 |  22       |  307 | `src/graph/algorithm/extract.rs`                 |
|  36 |  38       |  299 | `src/graph/algorithm/promote.rs`                 |
|  34 |  36       |  268 | `src/graph/algorithm/budget.rs`                  |
|  32 |   0       |  185 | `src/graph/traits.rs`                            |
|  32 |   6       |  198 | `src/spatial/plateau.rs`                         |
|  31 |  12       |  229 | `src/tree/gtree.rs`                              |
|  30 |  25       |  229 | `src/graph/algorithm/observe.rs`                 |
|  29 |  15       |  320 | `src/graph/algorithm/split.rs`                   |
|  29 |   4       |  190 | `src/spatial/plateau_basis.rs`                   |
|  27 |   0       |  171 | `src/traits/accumulator.rs`                      |
|  26 |  29       |  226 | `src/diagnostics/dot.rs`                         |
|  26 |  11       |   79 | `src/graph/algorithm/fmt.rs`                     |
|  23 |   0       |  151 | `src/traits/observation.rs`                      |
|  23 |   2       |  138 | `src/traits/proratable.rs`                       |
|  21 |   9       |  145 | `src/diagnostics/display.rs`                     |
|  21 |   4       |   96 | `src/traits/attenuatable.rs`                     |
|  19 |   2       |  194 | `src/spatial/node.rs`                            |
|  19 |   0       |  113 | `src/traits/inspectable.rs`                      |
|  19 |   1       |  101 | `src/traits/rng.rs`                              |
|  18 |  28       |  140 | `src/diagnostics/plateau_audit.rs`               |
|  15 |   8       |  157 | `src/spatial/contour_range.rs`                   |
|  14 |   0       |  143 | `src/spatial/view.rs`                            |
|  13 |   0       |   75 | `src/graph/algorithm/plateau/noop_tracker.rs`    |
|  12 |   5       |   62 | `src/graph/algorithm/sample.rs`                  |
|  12 |   0       |   62 | `src/traits/weighable.rs`                        |

---

## Functions with CC > 10 (sorted by CC descending)

> Cognitive outliers (Cog >> CC) indicate deeply nested code that McCabe underestimates.

| CC  | Cognitive | SLOC | Function                                   | File                                              |
| --- | --------- | ---- | ------------------------------------------ | ------------------------------------------------- |
|  41 | **95**    |  201 | `on_evict`                                 | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  26 |  46       |  147 | `normalize`                                | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  24 |  16       |  115 | `dump_plateaus`                            | `diagnostics/dump.rs`                             |
|  22 |  21       |  139 | `diagnose_missed_violation`                | `diagnostics/diagnostic.rs`                       |
|  20 |  40       |   90 | `decompose_basis`                          | `graph/algorithm/query.rs`                        |
|  17 |  23       |  110 | `observe`                                  | `graph/algorithm/observe.rs`                      |
|  17 |  11       |  128 | `place_basis_element`                      | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  17 |  39       |   75 | `build_plateaus`                           | `graph/algorithm/plateau/mod.rs`                  |
|  16 |  28       |   83 | `evict_candidates`                         | `graph/algorithm/budget.rs`                       |
|  16 |  17       |   72 | `escalate_after_promote`                   | `graph/algorithm/rebalance.rs`                    |
|  15 |  17       |   85 | `fixup_plateau`                            | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  14 |  26       |  102 | `resolve`                                  | `graph/algorithm/rebalance.rs`                    |
|  13 |  15       |   81 | `dump_gtree_dot`                           | `diagnostics/dot.rs`                              |
|  13 |  19       |   44 | `contour_steps`                            | `diagnostics/plateau_invariants.rs`               |
|  13 |  13       |   57 | `check_p_i1_i_keys_are_contour_steps`      | `diagnostics/plateau_invariants.rs`               |
|  13 |  18       |   74 | `rebalance`                                | `graph/algorithm/rebalance.rs`                    |
|  12 |  28       |   57 | `audit_plateau_consistency`                | `diagnostics/plateau_audit.rs`                    |
|  12 |  13       |   54 | `check_plateau_basis_consistency`          | `diagnostics/plateau_invariants.rs`               |
|  12 |  22       |   69 | `check_p_i1_ii_tile_contiguity`            | `diagnostics/plateau_invariants.rs`               |
|  12 |  25       |   70 | `check_p_i1_iii_run_contains_tile`         | `diagnostics/plateau_invariants.rs`               |
|  12 |  19       |   42 | `check_p_i5_thatch_depth`                  | `diagnostics/plateau_invariants.rs`               |
|  12 |  24       |   48 | `depth_attenuation_factors`                | `graph/algorithm/decay.rs`                        |
|  12 |  13       |  181 | `evict_tip`                                | `graph/algorithm/evict.rs`                        |
|  12 |  17       |   48 | `repair_p_i4`                              | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  12 |  16       |  109 | `debug_assert_mirror_consistency`          | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  12 |  15       |   80 | `skip_promote`                             | `graph/algorithm/promote.rs`                      |
|  11 |  13       |   54 | `check_p_i4_thatch_one_hop`                | `diagnostics/plateau_invariants.rs`               |
|  11 | **31**    |   98 | `on_catalytic_split`                       | `graph/algorithm/plateau/dynamic_tracker.rs`      |
|  11 |  18       |   39 | `push_leaf_removal_violations_with_config` | `graph/algorithm/violation_push.rs`               |

---

## CC Thresholds (Reference)

| CC Range | Risk Level                            |
| -------- | ------------------------------------- |
| 1–5      | Low — simple, easy to test            |
| 6–10     | Moderate — manageable                 |
| 11–20    | High — consider refactoring           |
| > 20     | Very High — hard to test and maintain |

---

## Test Coverage Baseline (2026-03-25)

> From `cargo llvm-cov`.  Files below 90 % line coverage are the primary targets
> for Phase 9.

| File                                        | Line Cover | Fn Cover |
| ------------------------------------------- | ---------: | -------: |
| `graph/algorithm/plateau.rs` (combined)     |     71.82% |   80.65% |
| `diagnostics/invariants.rs`                 |     76.53% |   95.65% |
| `graph/algorithm/evict.rs`                  |     77.09% |   81.82% |
| `graph/algorithm/rebalance.rs`              |     78.65% |   91.94% |
| `diagnostics/diagnostic.rs`                 |     84.87% |   96.30% |
| `graph/algorithm/query.rs`                  |     84.22% |   97.62% |
| `graph/algorithm/budget.rs`                 |     87.97% |   92.31% |
| `spatial/contour_range.rs`                  |     87.50% |  100.00% |

> All other files are at ≥ 90 % line coverage.

---

## How to re-run this analysis

```bash
# Install tool (if not present)
curl -L -o /tmp/rca.tar.gz \
  https://github.com/mozilla/rust-code-analysis/releases/download/v0.0.25/rust-code-analysis-linux-cli-x86_64.tar.gz
tar xzf /tmp/rca.tar.gz -C /tmp && cp /tmp/rust-code-analysis-cli ~/.local/bin/

# Generate JSON output
mkdir -p metrics-output
~/.local/bin/rust-code-analysis-cli -m -p src/ -O json --pr -o metrics-output/

# Extract function-level data (CC > 5)
python3 docs/clean-code-plan/extract-metrics.py
```
