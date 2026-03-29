# Refactoring Patterns - Quick Reference (Current)

Status-first summary of pattern work in this repository.

## Pattern Status

### Pattern 1: Tell-Don't-Ask split ownership

- Status: Completed
- Code: `src/graph/algorithm/split.rs`
- Result: split operations are `GvGraph` methods (`self.bootstrap_split`, `self.catalytic_split`)

### Pattern 2: Escalation context grouping

- Status: Completed
- Code: `src/graph/algorithm/rebalance/context.rs`, `src/graph/algorithm/rebalance/resolve.rs`
- Result: escalation/resolve orchestration uses `EscalationContext` + `VTreeMutContext`

### Pattern 3: Coordinate range abstraction

- Status: Completed
- Code: `src/spatial/range.rs`, query modules under `src/graph/algorithm/query/`
- Result: query recursion uses `CoordinateRange<C>` instead of loose lo/hi pairs

### Pattern 4: Violation queue ownership

- Status: Completed for orchestration call paths
- Code: `src/graph/algorithm/violation_push.rs`, `src/graph/algorithm/rebalance/resolve.rs`, `src/graph/algorithm/split/helpers.rs`, `src/graph/algorithm/evict.rs`
- Result: high-churn callers now use `ViolationQueue` methods; free-function wrappers remain for compatibility

### Pattern 5: Plateau signature flattening

- Status: Deferred
- Code: `src/graph/algorithm/plateau/dynamic_tracker/`
- Note: helper-phase extraction already reduced accidental complexity in key hotspots

## Validation Commands

```bash
cargo check --all-features
cargo test --all-features
```

## Risk Focus

- Violation propagation ordering must remain identical.
- Queue semantics must stay LIFO-compatible with current rebalance behavior.
- No additional residual violations after resolve loop.
