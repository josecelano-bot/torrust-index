# Clean-Code Refactor Plan

## Purpose

Systematically improve the `torrust-mudlark` codebase to make it **cleaner**,
**more maintainable**, **more sustainable**, and **more testable**.

This plan supersedes the Q/R-series refactor plan (archived in
`docs/archive/refactor-plan/`).  That plan focused on documentation and a first
pass at complexity reduction.  This plan goes deeper: it targets **structural
design debt**, **function decomposition**, **encapsulation**, and **test coverage
gaps** identified from a fresh complexity analysis run on 2026-03-28.

---

## Goals

| Goal                    | Success Metric                                                  |
| ----------------------- | --------------------------------------------------------------- |
| Reduce cognitive load   | No function with Cognitive Complexity > 20                      |
| Reduce cyclomatic risk  | No function with CC > 10 in production code                     |
| Improve testability     | All production-code functions individually unit-testable        |
| Improve encapsulation   | `GNode`/`VNode` fields inaccessible outside their modules       |
| Improve readability     | Every top-level function ≤ 40 SLOC, with a clear phase structure |
| Improve coverage        | Line coverage ≥ 90 % across all production files                |

## Non-Goals

- Changing the public API surface of `GvGraph` (breaking semver)
- Changing the fundamental algorithm (spatial partition tree + V-tree)
- Rewriting entire files from scratch without incremental verification

---

## Methodology

1. **Read before changing.** Each task begins with reading the targeted function
   body to understand what it does, not just how long it is.
2. **One step at a time.** Each commit changes exactly one logical thing.
3. **Test after every step.** Run `cargo test` before and after each change.
4. **Document the intent.** Every extracted helper function must have a one-line
   doc comment stating its purpose.
5. **Track metrics.** Each phase notes its before/after CC and cognitive scores.

---

## How to use this plan

- Open [baseline.md](baseline.md) to see the complexity numbers from the initial run.
- Pick the lowest-numbered **not-started** task in the highest-priority phase.
- Mark it **in-progress** before you start.
- After the commit, mark it **done** and update this file.
- After each phase completes, run the analysis again and update [baseline.md](baseline.md).

---

## Phase overview

| Phase | Name                       | Files targeted                                   | Priority |
| ----- | -------------------------- | ------------------------------------------------ | -------- |
| 1     | [Encapsulation](phase-1-encapsulation.md)     | `nodes/gnode.rs`, `nodes/vnode.rs`    | High     |
| 2     | [Plateau Tracker](phase-2-plateau-tracker.md) | `graph/algorithm/plateau/dynamic_tracker.rs`     | High     |
| 3     | [Rebalance](phase-3-rebalance.md)             | `graph/algorithm/rebalance.rs`        | High     |
| 4     | [Query](phase-4-query.md)                     | `graph/algorithm/query.rs`            | Medium   |
| 5     | [Diagnostics](phase-5-diagnostics.md)         | `diagnostics/invariants.rs`, `plateau_invariants.rs`, `diagnostic.rs`, `dump.rs` | Medium |
| 6     | [Observe / Evict / Budget](phase-6-observe-evict.md) | `algorithm/observe.rs`, `evict.rs`, `budget.rs` | Medium |
| 7     | [Nodes](phase-7-nodes.md)                     | `nodes/gnode.rs`, `nodes/vnode.rs` (structural clarity)  | Medium |
| 8     | [API & Traits](phase-8-api.md)                | `graph/traits.rs`, `spatial/`, trait files       | Low      |
| 9     | [Test Coverage](phase-9-testing.md)           | All files with line coverage < 90 %              | Medium   |

---

## Global progress

- [x] Phase 1 (Encapsulation) started
- [x] Phase 1 complete — complexity baseline updated
- [x] Phase 2 (Plateau Tracker) started
- [x] Phase 2 complete — complexity baseline updated
- [x] Phase 3 (Rebalance) started
- [x] Phase 3 complete — complexity baseline updated
- [x] Phase 4 (Query) started
- [x] Phase 4 complete — complexity baseline updated
- [x] Phase 5 (Diagnostics) started
- [x] Phase 5 complete — complexity baseline updated
- [x] Phase 6 (Observe/Evict/Budget) started
- [x] Phase 6 complete — complexity baseline updated
- [ ] Phase 7 (Nodes structural) started
- [ ] Phase 7 complete — complexity baseline updated
- [ ] Phase 8 (API & Traits) started
- [ ] Phase 8 complete — complexity baseline updated
- [ ] Phase 9 (Test Coverage) started
- [ ] Phase 9 complete — coverage baseline updated

---

## Rules

- Keep each step small and reversible.
- Use GPG-signed commits: `git commit -S`.
- Commit message format: `refactor(<scope>): <what and why>` following Conventional Commits.
- Run the full test suite before pushing: `cargo test`.
- Update this file after every phase completes.
