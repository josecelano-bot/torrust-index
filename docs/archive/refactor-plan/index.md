# Torrust-index Refactor Plan

## Purpose
Track a structured refactor of `torrust-index` for readability, maintainability, and testability.

## Workflow
1. Pick a file/area from the plan.
2. Apply small refactor step.
3. Run test(s): `cargo test` (targeted, then full).
4. Commit each step.
5. Update per-file progress.

## Global progress status
- [x] Initial baseline established
- [x] Refactor plan docs created
- [x] Core files refactored
- [x] All tests green

## File tracking
- [x] `src/lib.rs` -> see `lib.md`
- [x] `src/handle.rs` -> see `handle.md`
- [x] `src/arena.rs` -> see `arena.md`
- [x] `src/nodes/gnode.rs` -> see `nodes_gnode.md`
- [x] `src/nodes/vnode.rs` -> see `nodes_vnode.md`
- [x] `src/tree/gtree.rs` -> see `tree_gtree.md`
- [x] `src/tree/vtree.rs` -> see `tree_vtree.md`
- [x] `src/graph/config.rs` -> see `graph_config.md`
- [x] `src/graph/gv_graph.rs` -> see `graph_gv_graph.md`
- [x] `src/graph/traits.rs` -> see `graph_traits.md`
- [x] `src/traits` -> see `traits.md`
- [x] `src/spatial` -> see `spatial.md`
- [x] `src/diagnostics` -> see `diagnostics.md`
- [x] `tests/integration.rs` -> see `tests_integration.md`
- [x] `tests/snapshot_tests.rs` -> see `tests_snapshot.md`

## Rules
- Keep each refactor step small and reversible.
- Use meaningful commit messages.
- Update this index and file-level task file after each commit.
- Keep pre/post test results in the file notes.
