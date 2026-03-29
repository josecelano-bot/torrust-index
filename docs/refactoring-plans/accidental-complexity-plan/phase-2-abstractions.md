# Phase 2 - Abstraction Introduction

## Delivery Intent

Introduce small, focused abstractions that reduce cognitive load and duplication without changing behavior.

## Deliverables

1. Split flow phase helpers (preconditions, preprocess, allocation/wiring, finalize).
2. VTree mutation mini-API for repeated low-level patterns.
3. Optional narrow context structs only where signatures remain noisy.

## Step Checklist

### P2.1 Split flow phases

- [ ] Extract split precondition helper
- [ ] Extract parent-triple preprocess helper
- [ ] Extract child allocation and V wiring helper
- [ ] Extract finalize helper for evictable propagation and plateau update

### P2.2 VTree mutation mini-API

- [ ] Add or standardize `replace_child` style operation
- [ ] Add or standardize `remove_child` style operation
- [ ] Add or standardize recompute+propagate helper
- [ ] Replace repeated mutation snippets with named operations

### P2.3 Signature simplification

- [ ] Introduce focused context struct only where it clearly reduces noise
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
