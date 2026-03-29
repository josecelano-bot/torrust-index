# Cyclomatic Complexity Analysis

## Goal

Periodic cyclomatic complexity analysis of the `src/` directory using
[`rust-code-analysis-cli`](https://github.com/mozilla/rust-code-analysis) by Mozilla.

## Tool

**`rust-code-analysis-cli`** — computes 16 code metrics per file and per
function, including:

| Metric      | Description                                        |
| ----------- | -------------------------------------------------- |
| `cc`        | Cyclomatic Complexity (McCabe)                     |
| `cognitive` | Cognitive Complexity                               |
| `nom`       | Number of functions / closures                     |
| `nargs`     | Function argument count                            |
| `nexits`    | Number of exit points                              |
| `halstead`  | Halstead suite (effort, difficulty, bugs estimate) |
| `mi`        | Maintainability Index                              |

## Steps

### 1. Install

`cargo install rust-code-analysis-cli` fails on current Rust nightly (1.96) due to
a type-check regression in the crate. Use the pre-built Linux binary instead:

```bash
curl -L -o /tmp/rca.tar.gz \
  https://github.com/mozilla/rust-code-analysis/releases/download/v0.0.25/rust-code-analysis-linux-cli-x86_64.tar.gz
tar xzf /tmp/rca.tar.gz -C /tmp
cp /tmp/rust-code-analysis-cli ~/.local/bin/
rust-code-analysis-cli --version   # should print 0.0.25
```

### 2. Quick Console Dump

Human-readable overview, all metrics printed to stdout:

```bash
rust-code-analysis-cli -m -p src/
```

### 3. Full JSON Export

One JSON file per source file written to `./metrics-output/`:

```bash
mkdir -p metrics-output
rust-code-analysis-cli -m -p src/ -O json --pr -o metrics-output/
```

### 4. Extract High-Complexity Functions

Re-runnable `jq` script that walks the nested `spaces` tree and emits every
function unit sorted by CC descending:

```bash
find metrics-output -name "*.json" | xargs -I{} sh -c '
  FILE={}
  jq --arg f "$FILE" "
    def short(f): f | ltrimstr(\"metrics-output/\") | rtrimstr(\".json\");
    def walk_s:
      if type == \"object\" and has(\"name\") and has(\"metrics\") and has(\"kind\") then
        (if .kind == \"function\"
         then {file: (short(\$f)), name: .name,
               cc: (.metrics.cyclomatic.sum // 0),
               cognitive: (.metrics.cognitive.sum // 0),
               sloc: (.metrics.loc.sloc // 0)}
         else empty end),
        (if has(\"spaces\") then (.spaces // [])[] | walk_s else empty end)
      elif type == \"object\" then (to_entries[].value | walk_s)
      else empty end;
    walk_s
  " "\$FILE"
' | jq -s "sort_by(-.cc) | .[] | select(.cc > 10)" \
  | jq -r '"CC=\(.cc)  Cog=\(.cognitive)  SLOC=\(.sloc)  \(.name)  [\(.file)]"'
```

## Results (March 2026 — current baseline)

> Re-evaluated on **2026-03-29** from the current code state using the same
> `rust-code-analysis-cli` workflow described above.
>
> Fresh snapshot path: `metrics-output/recheck-2026-03-29/`

### Package-Wide Summary

| Metric                     | Value  |
| -------------------------- | ------ |
| Analyzed files under `src` | 98     |
| Total functions/closures   | 1101   |
| Total CC (sum)             | 2359   |
| Total Cognitive (sum)      | 1494   |
| Total SLOC                 | 17838  |
| Functions with CC > 10     | 29     |
| Functions with CC > 20     | 0      |

### Files by Aggregate Cyclomatic Complexity

| CC  | Cognitive | SLOC | File                                                          |
| --- | --------- | ---- | ------------------------------------------------------------- |
| 128 | 174       | 825  | `src/diagnostics/invariants.rs`                               |
| 128 | 192       | 628  | `src/diagnostics/plateau_invariants.rs`                       |
| 95  | 18        | 696  | `src/nodes/vnode.rs`                                          |
| 85  | 15        | 559  | `src/nodes/gnode.rs`                                          |
| 81  | 53        | 596  | `src/tree/vtree.rs`                                           |
| 73  | 54        | 645  | `src/graph/algorithm/rebalance.rs`                            |
| 68  | 104       | 361  | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs` |
| 67  | 52        | 378  | `src/graph/algorithm/violation_push.rs`                       |
| 65  | 0         | 523  | `src/traits/coordinate.rs`                                    |
| 63  | 7         | 585  | `src/graph/gv_graph.rs`                                       |
| 62  | 59        | 432  | `src/graph/algorithm/decay.rs`                                |
| 59  | 15        | 781  | `src/spatial/pewei/core.rs`                                   |
| 54  | 25        | 272  | `src/diagnostics/dump.rs`                                     |
| 53  | 39        | 295  | `src/diagnostics/diagnostic/tests.rs`                         |
| 51  | 7         | 415  | `src/arena.rs`                                                |
| 49  | 36        | 660  | `src/graph/algorithm/evict.rs`                                |
| 46  | 39        | 329  | `src/graph/algorithm/budget.rs`                               |
| 42  | 45        | 309  | `src/graph/algorithm/rebalance/resolve.rs`                    |
| 41  | 22        | 307  | `src/graph/algorithm/extract.rs`                              |
| 39  | 2         | 384  | `src/graph/algorithm/query.rs`                                |

> File-level CC is the **sum** of all functions / impls in that file.

### Functions with CC > 10 (sorted)

| CC  | Cognitive | SLOC | Function                                         | File                                                                     |
| --- | --------- | ---- | ------------------------------------------------ | ------------------------------------------------------------------------ |
| 20  | 40        | 64   | `decompose_basis`                                | `src/graph/algorithm/query/contour.rs`                                   |
| 17  | 39        | 75   | `build_plateaus`                                 | `src/graph/algorithm/plateau/read_api.rs`                                |
| 17  | 23        | 110  | `observe`                                        | `src/graph/algorithm/observe.rs`                                         |
| 16  | **49**    | 71   | `evict_ancestor_key`                             | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs`      |
| 15  | 18        | 75   | `evacuate_adjacent_plateaus`                     | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs`      |
| 15  | 17        | 85   | `fixup_plateau`                                  | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs`      |
| 15  | 14        | 105  | `normalize_impl`                                 | `src/graph/algorithm/plateau/dynamic_tracker/normalize.rs`               |
| 13  | 18        | 78   | `rebalance`                                      | `src/graph/algorithm/rebalance.rs`                                       |
| 13  | 15        | 81   | `dump_gtree_dot`                                 | `src/diagnostics/dot.rs`                                                 |
| 13  | 13        | 57   | `check_p_i1_i_keys_are_contour_steps`            | `src/diagnostics/plateau_invariants.rs`                                  |
| 13  | 8         | 67   | `dump_plateaus`                                  | `src/diagnostics/dump.rs`                                                |
| 12  | 32        | 50   | `collect_normalize_elements`                     | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs` |
| 12  | 28        | 57   | `audit_plateau_consistency`                      | `src/diagnostics/plateau_audit.rs`                                       |
| 12  | 26        | 45   | `contour_steps`                                  | `src/diagnostics/plateau_invariants.rs`                                  |
| 12  | 25        | 70   | `check_p_i1_iii_run_contains_tile`               | `src/diagnostics/plateau_invariants.rs`                                  |
| 12  | 24        | 48   | `depth_attenuation_factors`                      | `src/graph/algorithm/decay.rs`                                           |
| 12  | 22        | 69   | `check_p_i1_ii_tile_contiguity`                  | `src/diagnostics/plateau_invariants.rs`                                  |
| 12  | 19        | 42   | `check_p_i5_thatch_depth`                        | `src/diagnostics/plateau_invariants.rs`                                  |
| 12  | 17        | 76   | `on_evict_impl`                                  | `src/graph/algorithm/plateau/dynamic_tracker/evict.rs`                   |
| 12  | 17        | 48   | `repair_p_i4_impl`                               | `src/graph/algorithm/plateau/dynamic_tracker/repair.rs`                  |
| 12  | 16        | 109  | `debug_assert_mirror_consistency_impl`           | `src/graph/algorithm/plateau/dynamic_tracker/debug_diff.rs`              |
| 12  | 15        | 80   | `skip_promote`                                   | `src/graph/algorithm/promote.rs`                                         |
| 12  | 13        | 54   | `check_plateau_basis_consistency`                | `src/diagnostics/plateau_invariants.rs`                                  |
| 12  | 13        | 181  | `evict_tip`                                      | `src/graph/algorithm/evict.rs`                                           |
| 12  | 10        | 47   | `depth_two_entry_with_non_ancestor_collapse_sibling` | `src/diagnostics/diagnostic/tests.rs`                                |
| 11  | 31        | 103  | `on_catalytic_split_impl`                        | `src/graph/algorithm/plateau/dynamic_tracker/split_catalytic.rs`         |
| 11  | 18        | 39   | `push_leaf_removal_violations_with_config`       | `src/graph/algorithm/violation_push.rs`                                  |
| 11  | 17        | 63   | `evict_candidates`                               | `src/graph/algorithm/budget.rs`                                          |
| 11  | 13        | 54   | `check_p_i4_thatch_one_hop`                      | `src/diagnostics/plateau_invariants.rs`                                  |

### Key Observations (current baseline)

- The current maximum cyclomatic complexity at function level is
  `decompose_basis` (CC=20, Cog=40).
- No function currently exceeds CC > 20.
- Complexity is now distributed across **many medium-complexity helpers**,
  especially under `graph/algorithm/plateau/dynamic_tracker/*` and diagnostics.
- New highest cognitive outlier is `evict_ancestor_key` (CC=16, Cog=49), which
  suggests nesting/readability pressure despite moderate McCabe.
- File-level hotspots shifted from a single `plateau/mod.rs` concentration to
  diagnostics and node modules (`invariants.rs`, `plateau_invariants.rs`,
  `vnode.rs`, `gnode.rs`).
- The current scope still includes every `.rs` file under `src/`, including
  in-tree test modules such as `src/diagnostics/diagnostic/tests.rs`.

## Stabilization Refresh (2026-03-29, post-reorganization)

> Recomputed after the Phase 4 reorganization/dedup passes.
>
> Fresh snapshot path: `metrics-output/recheck-2026-03-29-stabilization/`

### Package-Wide Delta vs previous recheck

| Metric | Previous | Current | Delta |
| --- | ---: | ---: | ---: |
| Analyzed files under `src` | 98 | 102 | +4 |
| Total functions/closures | 1101 | 1133 | +32 |
| Total CC (sum) | 2359 | 2383 | +24 |
| Total Cognitive (sum) | 1494 | 1460 | -34 |
| Total SLOC | 17838 | 18131 | +293 |
| Functions with CC > 10 | 29 | 27 | -2 |
| Functions with CC > 20 | 0 | 0 | 0 |
| Max function CC | 20 | 17 | -3 |

### Current Top Function Hotspots (CC > 10)

| CC | Cognitive | SLOC | Function | File |
| ---: | ---: | ---: | --- | --- |
| 17 | 39 | 75 | `build_plateaus` | `src/graph/algorithm/plateau/read_api.rs` |
| 17 | 23 | 110 | `observe` | `src/graph/algorithm/observe.rs` |
| 16 | 49 | 71 | `evict_ancestor_key` | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs` |
| 15 | 18 | 75 | `evacuate_adjacent_plateaus` | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs` |
| 15 | 17 | 85 | `fixup_plateau` | `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/fixup.rs` |
| 15 | 14 | 105 | `normalize_impl` | `src/graph/algorithm/plateau/dynamic_tracker/normalize.rs` |

### Notes

- Acceptance criterion remains satisfied: no functions above CC 20.
- The previous top-CC hotspot (`decompose_basis`, CC=20) is no longer the max.
- Remaining pressure is now mostly cognitive (not McCabe), centered in plateau dynamic-tracker helpers.

### Suggested Next Refactor Targets

1. `evict_ancestor_key` (Cog=49): split nested control flow into phase helpers.
2. `decompose_basis` (CC=20, Cog=40): extract branch-specific decomposition
   strategies to reduce decision density.
3. `build_plateaus` (CC=17, Cog=39): separate traversal from output assembly.
4. `collect_normalize_elements` (CC=12, Cog=32): isolate filtering and merge
   policy branches.

## Thresholds (Reference)

| CC Range | Risk Level                            |
| -------- | ------------------------------------- |
| 1–5      | Low — simple, easy to test            |
| 6–10     | Moderate — manageable                 |
| 11–20    | High — consider refactoring           |
| > 20     | Very High — hard to test and maintain |

## Scope

- **Included**: all `.rs` files under `src/`
- **Excluded**: CI integration, automated enforcement, test files under `tests/`
