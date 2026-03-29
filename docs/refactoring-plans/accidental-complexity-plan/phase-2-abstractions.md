# Phase 2 - Abstraction Introduction

## Delivery Intent

Introduce small, focused abstractions that reduce cognitive load and duplication without changing behavior.

## Deliverables

1. Split flow phase helpers (preconditions, preprocess, allocation/wiring, finalize).
2. VTree mutation mini-API for repeated low-level patterns.
3. Optional narrow context structs only where signatures remain noisy.

## Step Checklist

### P2.1 Split flow phases

- [x] Extract split precondition helper
- [x] Extract parent-triple preprocess helper
- [x] Extract child allocation and V wiring helper
- [x] Extract finalize helper for evictable propagation and plateau update

### P2.2 VTree mutation mini-API

- [ ] Add or standardize `replace_child` style operation
- [ ] Add or standardize `remove_child` style operation
- [x] Add or standardize recompute+propagate helper
- [ ] Replace repeated mutation snippets with named operations

### P2.3 Signature simplification

- [x] Introduce focused context struct only where it clearly reduces noise
- [ ] Validate contexts do not become generic dumping containers

## Acceptance Criteria

- [ ] At least three hotspot functions become clearer or less complex
- [ ] Duplicated mutation or traversal snippets are reduced
- [ ] No behavior changes

## Test Checklist (per step)

- [ ] Gate A: `cargo check --all-features`
- [ ] Gate B: targeted tests for touched area
- [ ] Gate C: `cargo test --all-features`

## Progress Log

### Step Template

- Step:
- Status: [ ]
- Files:
- Commit:
- Tests:
  - [ ] Gate A
  - [ ] Gate B
  - [ ] Gate C
- Notes:
- Follow-up:

### 2026-03-29

- Step: P2.1 split flow phases
- Status: [x]
- Files: `src/graph/algorithm/split.rs`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Split flow now uses named helpers for candidate checks, parent preprocessing, child allocation, and shared entry cleanup.
- Follow-up: Continue P2.2 VTree mutation mini-API and keep helper scope narrow.

### 2026-03-29

- Step: P2.3 signature simplification
- Status: [x]
- Files: `src/graph/algorithm/rebalance/context.rs`, `src/graph/algorithm/rebalance/resolve.rs`, `src/spatial/range.rs`, `src/graph/algorithm/query/*.rs`
- Commit: [pending]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Added `EscalationContext`, `VTreeMutContext`, and `CoordinateRange` to remove repeated parameter groupings in rebalance and query code.
- Follow-up: Measure whether additional grouping is still warranted before expanding this pattern further.

### 2026-03-29

- Step: P2.3 signature simplification (rebalance resolve extension)
- Status: [x]
- Files: `src/graph/algorithm/rebalance/resolve.rs`
- Commit: `a3d2dce`
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Expanded `VTreeMutContext` usage from escalation-only helpers into `resolve_try_contract_parent` and `resolve_path_b`, reducing repeated `vnodes`/`violations` parameter threading across the resolve paths.
- Follow-up: Keep context scope narrow and avoid long-lived mutable aliases when borrowing through the context.

### 2026-03-29

- Step: P2.3 signature simplification (resolve entrypoint migration)
- Status: [x]
- Files: `src/graph/algorithm/rebalance/resolve.rs`, `src/graph/algorithm/rebalance.rs`
- Commit: [created]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Changed `resolve` to take `&mut VTreeMutContext` directly, so rebalance no longer threads `vnodes` and `violations` as separate arguments into the resolve entrypoint.
- Follow-up: Continue tightening rebalance helper signatures where context grouping remains clear and local.

### 2026-03-29

- Step: P2.2 VTree mutation mini-API (partial)
- Status: [ ]
- Files: `src/tree/vtree.rs`, `src/graph/algorithm/promote.rs`, `src/graph/algorithm/split.rs`, `src/graph/algorithm/evict.rs`
- Commit: [created]
- Tests:
  - [x] Gate A
  - [x] Gate B
  - [x] Gate C
- Notes: Introduced `recompute_and_sync_parent_slot` and `set_entry_flags`, then migrated promote/split/evict call sites.
- Follow-up: Standardize `replace_child` and `remove_child` operations to complete P2.2.
