# Refactoring Patterns – Metrics & Measurement (Updated)

This file tracks measurable outcomes for pattern work from the current repository state.

## Implemented Pattern Checks

### Pattern 1 and Pattern 2 completion indicators

- `EscalationContext` and `VTreeMutContext` are present and used in resolve path.
- Split orchestration is method-owned in `GvGraph` split implementation.

Quick checks:

```bash
rg "EscalationContext|VTreeMutContext" src/graph/algorithm/rebalance
rg "self\.bootstrap_split|self\.catalytic_split" src/graph/algorithm/split.rs
```

### Pattern 3 completion indicator

- `CoordinateRange<C>` exists and is used in query internals.

Quick checks:

```bash
rg "CoordinateRange" src/spatial src/graph/algorithm/query
```

## Remaining Pattern Metric: Pattern 4

Primary measurable target:

- Reduce direct orchestration usage of `&mut Vec<VNodeId>` in algorithm call chains.

Baseline probe:

```bash
rg "&mut Vec<VNodeId>" src/graph/algorithm
```

Post-slice probe:

```bash
rg "&mut Vec<VNodeId>" src/graph/algorithm/rebalance src/graph/algorithm/split
```

Success threshold for each slice:

- Fewer raw queue parameters in touched modules.
- No behavior regressions.

## Validation Pipeline

Run after each migration slice:

```bash
cargo check --all-features
cargo test --all-features
```

Optional complexity snapshot:

```bash
rust-code-analysis-cli -o metrics-output/ src/graph/algorithm/rebalance/resolve.rs
```

## Regression Signals

- New warnings or failures in rebalance/split paths.
- Residual violations observed after resolve loops.
- Any change in observed violation propagation order.
```

---

## Success Criteria

✅ Refactoring is successful if:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Avg function params (hotspots) | -40% | Count & compare |
| Tell-Don't-Ask violations | < 5 | Search free functions |
| Parameter pair occurrences | -75% | grep lo/hi, lo/hi patterns |
| CC (complexity) | Unchanged | rust-code-analysis |
| Test coverage | >= 95% | cargo tarpaulin |
| All tests pass | 100% | cargo test |

---

## Red Flags (Stop & Review)

⚠️ Refactoring should be paused if:

1. **Cyclomatic Complexity increases** by > 10%
   - Fix: Simplify context structure or split function

2. **Test coverage drops** below 90%
   - Fix: Add tests before refactoring

3. **New compiler warnings**
   - Fix: Resolve before proceeding

4. **Performance regression** (if benchmarked)
   - Fix: Profile and optimize context structures

5. **Parameter counts increase** in any hotspot
   - Fix: Review context design

