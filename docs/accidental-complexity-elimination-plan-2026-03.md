# Accidental Complexity Elimination Plan (2026-03)

## Purpose

Define a concrete, end-to-end plan to aggressively remove accidental complexity
while preserving behavior.

This plan targets three tracks in parallel:

1. Reduce complexity in current code paths.
2. Introduce missing abstractions where they lower cognitive load.
3. Reorganize modules to improve cohesion and navigability.

Execution tracker:

- [docs/accidental-complexity-execution-checklist-2026-03.md](docs/accidental-complexity-execution-checklist-2026-03.md)

## Scope

Included:

- `src/graph/algorithm/**`
- `src/tree/**`
- `src/diagnostics/**` (only accidental complexity reductions, no semantics changes)

Out of scope for this plan:

- Public API redesign unless required by internal refactor safety.
- Feature additions.

## What Counts As Accidental Complexity

A code region is treated as accidental complexity if one or more hold:

- Control flow is hard to follow relative to problem difficulty (high cognitive complexity, moderate CC).
- Ownership boundaries are blurred (functions act like methods but live as free functions).
- Internal coupling is exposed via low-level structures (`Arena`, raw node access) where domain methods would be clearer.
- Module boundaries hide intent (router files carrying heavy implementation or mixed concerns).

## Essential vs Accidental Decision Rule

Before each refactor, classify complexity:

- Essential: required by domain invariants, balancing rules, tree consistency, and feature constraints.
- Accidental: due to representation leakage, broad signatures, mixed responsibilities, and weak module boundaries.

A change is accepted only when it reduces accidental complexity without weakening invariant checks.

## Guiding Design Rules

1. Ownership-local behavior should be methods.
2. Cross-tree orchestration should stay in `GvGraph` methods.
3. `Arena` should be an implementation detail in most call sites.
4. Prefer narrow capability traits or context structs over long parameter lists.
5. Keep router modules thin (`mod.rs` should route/re-export, not host heavy logic).

## Direct Response To Identified Smells

### Smell A: functions taking `&mut GvGraph`

Example: `bootstrap_split` in [src/graph/algorithm/split.rs](src/graph/algorithm/split.rs).

Plan:

- Convert private free helpers that take `&mut GvGraph` into private methods on `impl GvGraph` when they are algorithm phases of one operation.
- Keep truly standalone pure helpers as free functions.
- Keep cross-tree orchestrators as `GvGraph` methods (not `GTree` or `VTree` methods).

Target conversions (first pass):

- `bootstrap_split(graph, g_id)` -> `self.bootstrap_split(g_id)`
- `catalytic_split(graph, g_id)` -> `self.catalytic_split(g_id)`

Rationale:

- Reduces argument threading and mental stack.
- Makes operation lifecycle explicit in one impl block.

### Smell B: functions taking `&Arena<VNode<V>>` / `&mut Arena<VNode<V>>`

Examples in [src/tree/vtree.rs](src/tree/vtree.rs):

- `propagate_v_sums`
- `sync_intensity_in_parent`
- `recompute_all_v_intensities`
- `v_depth`
- `is_ancestor`

Plan:

- Move these into `VTree` as canonical methods where feasible.
- Keep free-function wrappers temporarily (deprecated/internal) only during migration.
- Migrate call sites to method form before deleting wrappers.

Rationale:

- Keeps invariants close to owner type.
- Prevents arena-level API sprawl.

## Workstreams

## WS1 - Ownership/API Normalization (Methods vs Free Functions)

Goal: remove method-shaped free functions.

Steps:

1. Inventory all functions taking `&mut GvGraph`, `&GvGraph`, `&Arena<GNode<_>>`, `&Arena<VNode<_>>`.
2. Classify each as one of:
   - owner-local method,
   - cross-tree orchestrator method on `GvGraph`,
   - pure helper free function.
3. Convert in small batches by module.
4. Remove redundant wrappers.

Acceptance checks:

- Fewer owner-typed parameters in private helpers.
- No behavior changes.

## WS2 - Abstraction Introduction

Goal: replace repetitive low-level patterns with named domain operations.

Candidate abstractions:

1. `SplitFlow` phase helpers (internal to `GvGraph`):
   - precondition gate
   - preprocess parent triple
   - allocate child pair
   - wire V-structure
   - finalize flags and plateau update

2. `VTree` mutation mini-API:
   - `replace_child`
   - `remove_child`
   - `recompute_structural`
   - `propagate_after_local_change`

3. Read-only traversal adapters for diagnostics:
   - trait-backed visitors instead of repeated ad-hoc loops.

4. Optional context objects for long signatures:
   - `SplitContext`
   - `EvictContext`

Acceptance checks:

- Measurable drop in high-cognitive functions.
- Fewer duplicated traversal/mutation fragments.

## WS3 - Module Reorganization

Goal: enforce high cohesion and thin facades.

Target shape:

- Keep orchestrators in:
  - [src/graph/algorithm/split.rs](src/graph/algorithm/split.rs)
  - [src/graph/algorithm/evict.rs](src/graph/algorithm/evict.rs)
- Keep detailed mechanics in focused submodules:
  - `split/preconditions.rs`
  - `split/phases.rs`
  - `vtree/mutation.rs`
  - `vtree/traversal.rs`

Execution rule:

- Reorganize only after method/API normalization in the same area, so moves are mostly mechanical.

Acceptance checks:

- Main orchestrator files become shorter and phase-oriented.
- New modules each have one clear responsibility.

## WS4 - Complexity-Driven Backlog

Prioritize using both cyclomatic and cognitive complexity.

Initial top backlog:

1. `evict_ancestor_key` (high cognitive load)
2. `decompose_basis`
3. split flow (`attempt_split`, bootstrap/catalytic phases)
4. `VTree` mutation/propagation cluster

For each target function:

- establish baseline metrics,
- apply one structural extraction,
- rerun tests,
- rerun metrics,
- keep/refine/revert.

## Migration Strategy

Use 4 repeating cycles per target area:

1. Characterize: map responsibilities and invariants.
2. Encapsulate: move behavior to owner methods.
3. Extract: introduce phase helpers/contexts.
4. Flatten: reduce nesting and merge duplicated branches.

## Safety Net

Per change slice:

1. `cargo check --all-features`
2. targeted unit/integration tests for touched module
3. `cargo test --all-features`

Per milestone:

1. rerun complexity analysis
2. capture before/after metrics

## Milestones And Deliverables

### M1 (API normalization)

Deliverables:

- Method conversion map (old symbol -> new symbol).
- `split.rs` helpers converted to private `GvGraph` methods.
- `VTree` arena-style helper migration plan completed and started.

### M2 (abstractions)

Deliverables:

- `SplitFlow` and `VTree` mutation abstractions in place.
- At least 3 hotspot functions simplified.

### M3 (reorganization)

Deliverables:

- Algorithm and tree internals reorganized into cohesive submodules.
- Facade/orchestrator files reduced in size and responsibility.

### M4 (stabilization)

Deliverables:

- Updated complexity report.
- Residual essential-complexity map documenting why remaining complexity exists.

## Exit Criteria ("Accidental Complexity Removed")

This plan is considered complete when all criteria pass:

1. No private helper still takes `&mut GvGraph` unless it is intentionally an orchestrator method.
2. `Arena`-centric APIs are no longer the dominant private interface for tree operations.
3. Router/orchestrator modules are phase-readable and cohesive.
4. No function with CC > 20.
5. Cognitive complexity outliers have explicit essential-complexity justification or an active follow-up task.
6. Full tests pass with no behavior deltas.

## Work Tracking Template (per PR)

- Goal:
- Scope:
- Symbols migrated:
- Invariants protected:
- Tests run:
- Metrics before:
- Metrics after:
- Follow-ups:
