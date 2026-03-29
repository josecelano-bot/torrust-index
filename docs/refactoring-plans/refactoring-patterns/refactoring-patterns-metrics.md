# Refactoring Patterns - Metrics & Measurement (Updated)

This file tracks measurable outcomes for pattern work from the current repository state.

## Implemented Pattern Checks

### Pattern 1 and Pattern 2 indicators

- `EscalationContext` and `VTreeMutContext` are present and used in resolve path.
- Split orchestration is method-owned in `GvGraph` split implementation.

Quick checks:

```bash
rg "EscalationContext|VTreeMutContext" src/graph/algorithm/rebalance
rg "self\.bootstrap_split|self\.catalytic_split" src/graph/algorithm/split.rs
```

### Pattern 3 indicator

- `CoordinateRange<C>` exists and is used in query internals.

Quick checks:

```bash
rg "CoordinateRange" src/spatial src/graph/algorithm/query
```

### Pattern 4 indicator

- High-churn orchestration paths use `ViolationQueue` methods.
- Raw queue mutation vectors in algorithm path are limited to compatibility internals in `violation_push.rs`.

Quick checks:

```bash
rg "ViolationQueue" src/graph/algorithm/rebalance src/graph/algorithm/split src/graph/algorithm/evict src/graph/algorithm/violation_push.rs
rg "&mut Vec<VNodeId>" src/graph/algorithm
```

## Validation Pipeline

```bash
cargo check --all-features
cargo test --all-features
```

## Regression Signals

- New failures in rebalance/split/evict paths.
- Residual violations observed after resolve loops.
- Any change in observed violation propagation order.
