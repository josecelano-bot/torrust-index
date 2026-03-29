# Refactoring Patterns - Implementation Roadmap (Updated)

This roadmap reflects the current repository state.

## Completed Phases

### Phase A: Tell-Don't-Ask split ownership

- Completed in `src/graph/algorithm/split.rs`.
- `bootstrap_split` and `catalytic_split` are implemented as `GvGraph` methods.

### Phase B: Escalation context grouping

- Completed in `src/graph/algorithm/rebalance/context.rs` and `src/graph/algorithm/rebalance/resolve.rs`.
- `EscalationContext` and `VTreeMutContext` are active in escalation/resolve orchestration.

### Phase C: CoordinateRange rollout

- Completed in query path and shared spatial utilities.
- `CoordinateRange<C>` is in `src/spatial/range.rs` and used by contour/range-sum query internals.

### Phase D: ViolationQueue migration

- Completed for high-churn orchestration paths.
- `ViolationQueue` wrapper added and used in:
  - `src/graph/algorithm/rebalance/resolve.rs`
  - `src/graph/algorithm/split/helpers.rs`
  - `src/graph/algorithm/evict.rs`
- Compatibility free-function wrappers are intentionally retained in `violation_push.rs`.

## Optional Phase

### Phase E: Plateau context flattening (deferred)

Deferred by ROI. Re-open only if new metrics show a clear complexity payoff.

## Validation Gates

1. `cargo check --all-features`
2. Targeted tests for touched module
3. `cargo test --all-features`

## Exit Criteria

- Pattern 4 completed for primary orchestration paths.
- No behavior regressions in violation propagation.
- Documentation synchronized with implementation.
