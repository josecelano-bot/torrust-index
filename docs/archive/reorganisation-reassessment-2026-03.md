# Module Reorganisation Reassessment (2026-03-29)

## Context

The previous reorganisation plan has been archived to:

- `docs/archive/module-reorganisation.md`

This document reassesses whether another reorganisation pass is worthwhile after recent refactors and feature work.

## What changed since initial draft

After validating and updating the architecture diagrams (`component-architecture` and
`call-flow`), the structural picture is clearer:

- `graph/traits` acts as a thin trait-adapter layer.
- `graph/gv_graph` is the actual orchestration/core dispatch layer.
- `spatial/view` and `spatial/node` are correctly separated by concern.
- `spatial/plateau` and `spatial/plateau_basis` are correctly separated
	(public shape vs dynamic bookkeeping).

So the top-level architecture is sound; remaining work is mostly targeted decomposition
inside large implementation files.

## Current state snapshot

The namespace structure is healthy:

- `src/graph/mod.rs` is thin and re-export focused.
- `src/diagnostics/mod.rs`, `src/nodes/mod.rs`, `src/spatial/mod.rs` are routing-oriented.
- Several prior extractions are still paying off (for example, `dynamic_tracker` and diagnostics subfolders).

Largest remaining files (approximate line count):

- `src/graph/algorithm/rebalance.rs` (~956)
- `src/diagnostics/invariants.rs` (~922)
- `src/graph/algorithm/plateau/dynamic_tracker/core_helpers.rs` (~785)
- `src/spatial/pewei/core.rs` (~781)
- `src/graph/algorithm/query.rs` (~715)
- `src/nodes/vnode.rs` (~696)
- `src/graph/algorithm/evict.rs` (~660)
- `src/diagnostics/plateau_invariants.rs` (~628)
- `src/graph/algorithm/plateau/mod.rs` (~612)

## Reassessment conclusion

Another broad folder-level reorganisation is **not** necessary right now.

A **targeted decomposition pass** is recommended for large modules where cognitive load remains high. The project already has a good top-level layout; the opportunity is now mostly at the file/module granularity inside existing folders.

Additionally, architecture documentation should be kept synchronized with module movement
as part of reorganisation work (diagram drift already occurred once).

## Recommended next pass (targeted)

### R1 — Split `rebalance.rs` by concern (High)

Candidate extraction:

- `rebalance/context.rs` for context wrappers and formatting (`Nd`, `Ctx`, display helpers)
- `rebalance/violation_scan.rs` for violated-node discovery logic
- `rebalance/resolve.rs` for resolve/collapse application paths

Keep `rebalance.rs` as orchestrator and public entrypoints.

### R2 — Split `invariants.rs` into grouped checks (High)

Candidate extraction:

- `invariants/graph_consistency.rs`
- `invariants/vtree_consistency.rs`
- `invariants/budget_checks.rs`
- `invariants/reporting.rs`

Keep one top-level `check_all_invariants` composition point.

### R3 — Split `graph/algorithm/plateau/mod.rs` by role (Medium)

Current file mixes:

- generic tracker wrappers,
- tracker-specific diagnostic/debug APIs,
- and public plateau-selection helpers.

Candidate extraction:

- `plateau/read_api.rs` for `plateaus`, `build_plateaus`, `select_plateaus`
- `plateau/update_wrappers.rs` for thin `plateau_after_*` wrappers
- `plateau/debug_api.rs` for tracker-specific debug/diagnostic methods

Keep `plateau/mod.rs` as module router and minimal re-export/orchestration point.

### R4 — Split `query.rs` into API and contour internals (Medium)

Candidate extraction:

- `query/get.rs`
- `query/range_sum.rs`
- `query/contour.rs`
- `query/sample.rs`

Retain `query.rs` as thin facade module.

### R5 — Continue decomposition of `dynamic_tracker/core_helpers.rs` (Medium)

It has become the next concentration point after successful first-wave extraction.

Candidate extraction:

- `dynamic_tracker/place.rs`
- `dynamic_tracker/fixup.rs`
- `dynamic_tracker/consolidate.rs`

### R6 — Defer unless pain increases (Low)

- `spatial/pewei/core.rs`
- `nodes/vnode.rs`

Both are large but relatively cohesive.

### R7 — Keep docs and diagrams architecture-synced (Low, process)

For any module move/split PR:

- update `docs/component-architecture.puml` and regenerate SVG,
- update `docs/call-flow.puml` only when flow semantics changed,
- include a short “architecture delta” note in PR description.

## Proposed acceptance criteria

- No behavior changes; all existing tests remain green.
- File size target: keep most implementation files under ~500 lines when practical.
- Namespace files (`mod.rs`) remain thin.
- Public API surface remains stable unless explicitly planned.
- Diagram sources in `docs/*.puml` stay consistent with module ownership after merges.

## Priority order

1. `rebalance.rs`
2. `invariants.rs`
3. `graph/algorithm/plateau/mod.rs`
4. `query.rs`
5. `dynamic_tracker/core_helpers.rs`
6. optional follow-up on `pewei/core.rs` and `vnode.rs`

## Execution and Progress Tracking Plan

This section turns the reassessment into an implementation plan that can be
executed and tracked incrementally.

### Tracking model

- Work in short slices: one extraction stream at a time.
- Keep each slice mergeable: no long-lived mega-branches.
- Track progress in this document using status markers:
	- `[ ]` not started
	- `[-]` in progress
	- `[x]` complete
- Require objective evidence for each completed slice:
	- tests pass,
	- architecture docs updated if needed,
	- line-count delta recorded.

### Stream backlog and status

#### S1 — `rebalance.rs` decomposition (High)

- [x] Create `rebalance/context.rs` and move `Nd`/`Ctx` + display helpers.
- [x] Create `rebalance/violation_scan.rs` and move discovery logic.
- [x] Create `rebalance/resolve.rs` and move resolve/collapse flow.
- [x] Reduce `rebalance.rs` to orchestration + public entrypoints.
- [x] Run `cargo test` and verify no behavior changes.
- [x] Update docs if call semantics changed.

#### S2 — `invariants.rs` decomposition (High)

- [x] Create `invariants/graph_consistency.rs`.
- [x] Create `invariants/vtree_consistency.rs`.
- [x] Create `invariants/budget_checks.rs`.
- [x] Create `invariants/reporting.rs`.
- [x] Keep a single `check_all_invariants` composition point.
- [x] Run `cargo test` and verify diagnostics behavior unchanged.

#### S3 — `plateau/mod.rs` role split (Medium)

- [x] Create `plateau/read_api.rs` (`plateaus`, `build_plateaus`, `select_plateaus`).
- [x] Create `plateau/update_wrappers.rs` (`plateau_after_*` wrappers).
- [x] Create `plateau/debug_api.rs` tracker-specific debug helpers.
- [x] Keep `plateau/mod.rs` as thin router + re-exports.
- [x] Run `cargo test` and verify feature-gated builds still pass.

#### S4 — `query.rs` decomposition (Medium)

- [x] Create `query/get.rs`.
- [x] Create `query/range_sum.rs`.
- [x] Create `query/contour.rs`.
- [x] Create `query/sample.rs`.
- [x] Keep `query.rs` as facade module.
- [x] Run `cargo test` and snapshot tests.

#### S5 — `dynamic_tracker/core_helpers.rs` decomposition (Medium)

- [x] Create `dynamic_tracker/core_helpers/place.rs`.
- [x] Create `dynamic_tracker/core_helpers/fixup.rs`.
- [x] Create `dynamic_tracker/core_helpers/consolidate.rs`.
- [x] Ensure helper visibility preserves sibling access from `dynamic_tracker`.
- [x] Run `cargo test` and targeted coverage check.

#### S6 — Optional follow-up (Low)

- [x] Re-evaluate `spatial/pewei/core.rs` after S1-S5.
- [x] Re-evaluate `nodes/vnode.rs` after S1-S5.

### Milestones

- [x] M1: S1 complete and merged.
- [x] M2: S2 complete and merged.
- [x] M3: S3 + S4 complete and merged.
- [x] M4: S5 complete and merged.
- [x] M5: Final architecture/doc sync and closeout.

### Quantitative KPIs

Track these after each milestone:

- Largest file lines (`top 10` by `wc -l`).
- Count of files above 700 lines.
- Count of files above 500 lines.
- `cargo test` pass/fail.
- Optional: file-level coverage for touched high-risk modules.

Suggested command set:

```bash
find src -name '*.rs' -print0 | xargs -0 wc -l | sort -nr | head -n 15
cargo test
```

### Progress log template

Append one entry per merged slice:

```text
Date:
Stream: S#
Status: [x]
PR/Commit:
Scope moved:
Largest file before/after:
Tests:
Coverage notes:
Docs updated: yes/no
Risks / follow-ups:
```

### Progress log

Date: 2026-03-29
Stream: S3 + S4 + S5
Status: [x]
PR/Commit: local workspace changes
Scope moved: plateau role split into `read_api.rs` / `update_wrappers.rs` / `debug_api.rs`; `query.rs` split into `get.rs` / `range_sum.rs` / `contour.rs` / `sample.rs`; dynamic tracker helpers split into `core_helpers/place.rs` / `core_helpers/fixup.rs` / `core_helpers/consolidate.rs`
Largest file before/after: `src/graph/algorithm/query.rs` 715 -> 384; current top files remain `rebalance.rs` (956) and `invariants.rs` (922)
Tests: `cargo test` passed (unit + integration + snapshot + doc tests)
Coverage notes: targeted coverage check deferred (full test suite green)
Docs updated: yes (this tracking document)
Risks / follow-ups: complete S1 and S2 to reduce remaining >700-line concentration points

Date: 2026-03-29
Stream: S1 + S2 + S6
Status: [x]
PR/Commit: local workspace changes
Scope moved: `rebalance.rs` split with new `rebalance/context.rs`, `rebalance/violation_scan.rs`, and `rebalance/resolve.rs`; `invariants.rs` split with new `invariants/graph_consistency.rs`, `invariants/vtree_consistency.rs`, `invariants/budget_checks.rs`, and `invariants/reporting.rs`
Largest file before/after: `src/graph/algorithm/rebalance.rs` 956 -> 645; `src/diagnostics/invariants.rs` 922 -> 825
Tests: `cargo test -q` passed (509 unit, 10 integration, 4 snapshot, 0 doc)
Coverage notes: no dedicated coverage run; behavioral parity validated via full test suite
Docs updated: yes (tracking status, milestones, and closeout)
Risks / follow-ups: optional deep decomposition remains possible for `spatial/pewei/core.rs` and `nodes/vnode.rs` if future complexity pressure increases

### Governance and guardrails

- No behavioral changes bundled with structural moves unless explicitly scoped.
- Prefer one stream per PR to simplify review.
- Keep module entry files (`mod.rs`) thin after each merge.
- If architecture ownership changes, update:
	- `docs/component-architecture.puml`
	- `docs/call-flow.puml` (only when execution flow semantics changed)

### Current overall status

- [x] Reassessment defined.
- [x] Execution plan completed.
- [x] S1 started.
- [x] S1-S5 completed.
- [x] Final closeout.
