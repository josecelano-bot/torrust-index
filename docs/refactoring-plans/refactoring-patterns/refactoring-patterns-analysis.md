# Refactoring Patterns Analysis (Current)

This analysis captures what is solved and what remains intentionally deferred.

## Completed Pattern Outcomes

### Pattern 1: Tell-Don't-Ask for split operations

- Split behavior is owned by `GvGraph` methods.
- Call sites use `self.bootstrap_split(...)` and `self.catalytic_split(...)`.

Evidence:
- `src/graph/algorithm/split.rs`

### Pattern 2: Context grouping for escalation path

- Escalation helper flow uses explicit context structs.
- Multi-parameter orchestration signatures are replaced by named contexts.

Evidence:
- `src/graph/algorithm/rebalance/context.rs`
- `src/graph/algorithm/rebalance/resolve.rs`

### Pattern 3: Coordinate range abstraction

- Query internals use `CoordinateRange<C>` in recursive paths.
- Loose lo/hi parameter pairing in core query recursion is eliminated.

Evidence:
- `src/spatial/range.rs`
- `src/graph/algorithm/query/range_sum.rs`
- `src/graph/algorithm/query/contour.rs`

### Pattern 4: Violation queue ownership for orchestration paths

- Added `ViolationQueue` wrapper in `violation_push.rs`.
- Migrated high-churn orchestration call sites to queue methods.

Evidence:
- `src/graph/algorithm/violation_push.rs`
- `src/graph/algorithm/rebalance/resolve.rs`
- `src/graph/algorithm/split/helpers.rs`
- `src/graph/algorithm/evict.rs`

Compatibility note:
- Existing free push functions are retained as compatibility wrappers.

## Deferred Pattern

### Pattern 5: Plateau context flattening

Deferred intentionally.

Rationale:
- Dynamic-tracker helper-phase extraction already reduced the highest accidental hotspots.
- Additional signature flattening does not currently justify risk/effort.
