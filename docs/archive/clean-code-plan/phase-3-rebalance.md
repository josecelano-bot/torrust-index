# Phase 3 — Rebalance Decomposition

## Goal

Break down the monolithic `rebalance.rs` into well-named, individually-testable
helper functions that each have a single, documentable responsibility.

## File targeted

- `src/graph/algorithm/rebalance.rs`

## Complexity baseline (2026-03-28)

| CC  | Cognitive | SLOC | Function                   |
| --- | --------- | ---- | -------------------------- |
|  16 |  17       |   72 | `escalate_after_promote`   |
|  14 |  26       |  102 | `resolve`                  |
|  13 |  18       |   74 | `rebalance`                |

File aggregate: **CC=110, Cog=107, 739 SLOC**.

## Target (post-Phase 3)

| Metric            | Before | Target |
| ----------------- | -----: | -----: |
| File CC           |    110 |  ≤ 60  |
| File Cognitive    |    107 |  ≤ 60  |
| Max function CC   |     16 |  ≤ 8   |
| Max function Cog  |     26 |  ≤ 15  |

---

## Background

`rebalance.rs` contains the V-tree rebalancing logic: contraction, promotion,
skip-promotion, and their orchestration.  The file is factored into functions,
but the three hotspots (`escalate_after_promote`, `resolve`, `rebalance`) each
mix guard logic, dispatch, and side-effect propagation in a single body.

The dominant cognitive hotspot is `resolve` (CC=14, Cog=26): it dispatches
between two structural paths ("standard promote" and "skip/legacy promote"),
but the dispatch logic and the path bodies are interleaved rather than clearly
separated.

The `escalate_after_promote` function (CC=16) has four clearly labelled phases
inside inline comments, making it a good candidate for direct extraction without
first needing annotations.

---

## Tasks

### P3.1 — Extract `escalate_after_promote` phase helpers

**Status:** `[x]` done

**What to do:**

`escalate_after_promote` already has four banner-commented phases.  Extract each
into a private helper:

| Helper fn name                          | Phase | Responsibility                                    |
| --------------------------------------- | ----- | ------------------------------------------------- |
| `escalate_identify_violation`           | 1     | Returns the heaviest child and violation class    |
| `escalate_contract_parent`              | 2     | Contracts 3-child parent; collects side-effect violations |
| `escalate_contract_grandparent`         | 3     | Optionally contracts grandparent `g`              |
| `escalate_skip_promote_fallback`        | 4     | Skip-promote fallback when contractions fail      |

After extraction the outer `escalate_after_promote` body should be ≤ 25 SLOC
and read as: identify → contract-parent → optionally-contract-grandparent →
fallback.

**Acceptance:** `cargo test` passes; `escalate_after_promote` CC ≤ 6, Cog ≤ 10.

---

### P3.2 — Annotate and clarify `resolve` dispatch paths

**Status:** `[x]` done

**Depends on:** P3.1

**What to do:**

1. Read `resolve` top-to-bottom.
2. Identify the two dispatch paths (Path A: standard promote; Path B: skip/legacy
   promote) and the pre-dispatch preliminaries (parent contraction attempt).
3. Build a `ResolveOutcome` or `PromotePath` enum so the dispatch reads as a
   `match` rather than nested `if/else`.
4. Extract the path logic into helpers:

| Helper fn name              | Responsibility                                         |
| --------------------------- | ------------------------------------------------------ |
| `try_contract_parent_first` | Preliminary parent contraction; returns whether resolved |
| `resolve_path_a`            | Standard-promote path (2-child structural)            |
| `resolve_path_b`            | Skip/legacy promote path                              |

**Acceptance:** `resolve` CC ≤ 7, Cog ≤ 12; `cargo test` passes.

---

### P3.3 — Simplify `rebalance` loop structure

**Status:** `[x]` done

**Depends on:** P3.2

**Baseline:** `rebalance` CC=13, Cog=18, SLOC=74.

**What to do:**

`rebalance` is a loop that drains a violation queue.  Its complexity comes from
handling the new-G-node side effect returned by `resolve` (the legacy-promote
path).  Clarify by:

1. Extracting the `resolve`-result-handling block into `handle_resolve_side_effects`.
2. Replacing comment-delimited sub-blocks with named `let` bindings that read as
   documentation.
3. Ensuring the loop termination invariant is documented in a `//` comment
   immediately above the loop.

**Acceptance:** `rebalance` CC ≤ 8, Cog ≤ 10; `cargo test` passes.

---

### P3.4 — Add targeted unit tests for extracted helpers

**Status:** `[ ]` not started

**Depends on:** P3.3

**What to do:**

Write at least one focused unit test for each of the helpers extracted in P3.1
and P3.2.  Use the existing `fresh()` / `make_config()` scaffolding already
present in `rebalance.rs`'s `#[cfg(test)]` module.

Test scenarios to cover:
- `try_contract_parent_first`: parent with 3 children that contracts successfully
  resolving the violation.
- `resolve_path_a`: standard promote leaves no violation.
- `resolve_path_b`: skip-promote fallback increments the violation queue.

**Acceptance:** ≥ 1 unit test per helper; `cargo test` passes.

---

## Commit message template

```
refactor(rebalance): <what>

<why — reference cognitive complexity numbers and the specific helper extracted>

Part of docs/clean-code-plan/phase-3-rebalance.md task P3.x.
```
