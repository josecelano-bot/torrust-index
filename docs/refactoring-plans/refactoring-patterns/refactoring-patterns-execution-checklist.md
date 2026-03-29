# Refactoring Patterns - Execution Checklist (Current)

This checklist reflects the current code state.

## Completed Items

- [x] Pattern 1: split tell-don't-ask migration
- [x] Pattern 2: escalation context grouping
- [x] Pattern 3: `CoordinateRange<C>` adoption in query path
- [x] Pattern 4: ViolationQueue migration in high-churn orchestration paths

## Pattern 4 Completion Details

### Step 4.1: Introduce queue facade

- [x] Added `ViolationQueue` wrapper around `Vec<VNodeId>` in violation-push layer.
- [x] Added method aliases for key push operations.
- [x] Kept existing free functions for compatibility.

Acceptance:

- [x] Compiles without behavior change.

### Step 4.2: Migrate rebalance resolve path

- [x] Migrated `src/graph/algorithm/rebalance/resolve.rs` call sites to queue methods.

Acceptance:

- [x] `cargo test --all-features` passes.
- [x] No residual-violation regressions observed in existing debug audits.

### Step 4.3: Migrate split helper path

- [x] Migrated `src/graph/algorithm/split/helpers.rs` queue call sites.

Acceptance:

- [x] Split-specific tests continue to pass.

### Step 4.4: Migrate secondary call sites

- [x] Migrated `src/graph/algorithm/evict.rs` orchestration call path.

Acceptance:

- [x] No high-churn call paths still pass raw queue mutation vectors.

## Deferred Checklist: Pattern 5 (Plateau Context)

- [x] ROI check performed.
- [x] Deferred unless new complexity metrics justify re-opening.

## Validation Gates

- [x] `cargo check --all-features`
- [x] Targeted tests for touched modules
- [x] `cargo test --all-features`
