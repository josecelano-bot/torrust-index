# Refactoring Patterns - Documentation Index (Current State)

This directory tracks pattern-driven complexity reduction in the algorithm layer.

## Status Snapshot

| Pattern | Status | Evidence in Code | Notes |
|---|---|---|---|
| Pattern 1: Tell-Don't-Ask for split operations | Completed | `src/graph/algorithm/split.rs` | `bootstrap_split` and `catalytic_split` are methods on `GvGraph` |
| Pattern 2: Escalation context grouping | Completed | `src/graph/algorithm/rebalance/context.rs`, `src/graph/algorithm/rebalance/resolve.rs` | `EscalationContext` and `VTreeMutContext` are in active use |
| Pattern 3: Coordinate range type | Completed | `src/spatial/range.rs`, `src/graph/algorithm/query/range_sum.rs`, `src/graph/algorithm/query/contour.rs` | Query recursion uses `CoordinateRange<C>` |
| Pattern 4: Violation queue ownership | Completed (compatibility wrappers retained) | `src/graph/algorithm/violation_push.rs`, `src/graph/algorithm/rebalance/resolve.rs`, `src/graph/algorithm/split/helpers.rs`, `src/graph/algorithm/evict.rs` | High-churn orchestration paths migrated to `ViolationQueue` methods |
| Pattern 5: Plateau context flattening | Deferred by ROI | `src/graph/algorithm/plateau/dynamic_tracker/` | Helper-phase decomposition already reduced major hotspots |

## Reading Order

1. [refactoring-patterns-quick-reference.md](refactoring-patterns-quick-reference.md)
2. [refactoring-patterns-analysis.md](refactoring-patterns-analysis.md)
3. [refactoring-patterns-execution-checklist.md](refactoring-patterns-execution-checklist.md)
4. [refactoring-patterns-metrics.md](refactoring-patterns-metrics.md)
5. [refactoring-patterns-roadmap.md](refactoring-patterns-roadmap.md)
6. [function-signature-inventory.md](function-signature-inventory.md)

## Validation Gates

- `cargo check --all-features`
- Targeted tests for touched area
- `cargo test --all-features`

## Current Outcome

- Pattern work is complete for implemented scope (Patterns 1-4).
- Pattern 5 remains intentionally deferred unless new metrics justify re-opening it.
