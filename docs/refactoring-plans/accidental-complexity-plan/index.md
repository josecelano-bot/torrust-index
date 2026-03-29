# Accidental Complexity Plan Index

## 🎯 Goal

Remove accidental complexity in small, safe, test-guarded increments while preserving behavior.

> 🔗 **Part of unified refactoring initiative.** Start here: [../index.md](../index.md)
> 
> **Before starting any phase:** Read [../PREREQUISITES.md](../PREREQUISITES.md) for blockers & sequencing.

## Plan Structure

- Phase 1: API normalization (method vs free function boundaries)
- Phase 2: abstraction introduction (phase helpers and focused mutation APIs)
- Phase 3: module reorganization (cohesion and discoverability)
- Phase 4: stabilization and proof (metrics and residual complexity map)

## How To Execute

1. Do one micro-change per commit.
2. Keep each commit behavior-preserving.
3. Run all test gates after each micro-change.
4. Mark progress in the phase file before moving to the next step.

## Test Gates (required for every step)

- Gate A (fast): `cargo check --all-features`
- Gate B (targeted): run focused tests for touched area
- Gate C (full): `cargo test --all-features`

A step is done only when Gate A + Gate B + Gate C pass.

## Commit Guardrails

- One focused goal per commit
- Prefer 1 to 3 touched files per commit
- Avoid mixing mechanical migration with logic changes

## Refactoring Discovery Tools

### Function Signature Inventory
**[function-signature-inventory.md](../refactoring-patterns/function-signature-inventory.md)** – Complete listing of all module, type, and function signatures in the torrust-index package.

Use this tool to:
- Identify opportunities for API normalization (inconsistent naming, redundant overloads)
- Spot functions with suspicious signatures (too many parameters, unclear ownership)
- Find candidates for abstraction (related functions that should be methods)
- Track function complexity markers (parameter count, mutability patterns)
- Make refactoring decisions based on current API surface

**Strategy:** Review this inventory when planning each phase to highlight accidental complexity patterns.

> 💡 **Note:** This tool is co-located with the Refactoring Patterns suite in `../refactoring-patterns/` for easier cross-reference when making optimization decisions.

## Tracking Links

**Core coordination:**
- [../index.md](../index.md) – Master navigation (START HERE)
- [../PREREQUISITES.md](../PREREQUISITES.md) – Blockers & sequencing
- [../PROGRESS.md](../PROGRESS.md) – Track overall status
- [../INTEGRATION.md](../INTEGRATION.md) – When to apply patterns
- [../ROLLBACK.md](../ROLLBACK.md) – Stop conditions & recovery

**Phase Details:**

## Global Exit Criteria

1. No private helper still takes mutable graph owner unless intentionally orchestrating cross-tree flow.
2. Arena-centric helper APIs are no longer dominant at call sites.
3. Orchestrator files are phase-readable and cohesive.
4. No function above CC 20.
5. Cognitive outliers are justified as essential or have explicit follow-up.
6. Full test suite passes.

## Current Snapshot

- Overall status: [ ] Not started
- Active phase: [ ] P1 / [ ] P2 / [ ] P3 / [ ] P4
- Last completed step:
- Current branch:
- Last successful full test run:
