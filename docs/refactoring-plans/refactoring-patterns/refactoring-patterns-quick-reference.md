# Refactoring Patterns – Quick Reference

One-page summary of actionable refactoring opportunities.

---

## Top 5 Patterns to Fix

### 🔴 Pattern 1: Tell-Don't-Ask (Free Functions → Methods)
**Severity:** CRITICAL | **Locations:** 2-5 | **Effort:** 1 day

Functions that ask for mutable graph and perform self-operations:

```rust
// WRONG: bootstrap_split(graph, id)
// RIGHT: graph.bootstrap_split(id)

bootstrap_split()          → GvGraph::bootstrap_split()
catalytic_split()          → GvGraph::catalytic_split()
```

**Files:** `src/graph/algorithm/split.rs`

---

### 🔴 Pattern 2: Many Parameters – Context Structures
**Severity:** CRITICAL | **Locations:** 3-5 | **Effort:** 2 days

5-7 parameter functions with implicit grouping:

```rust
// BEFORE (5-7 params)
escalate_contract_parent(vnodes, p, heaviest, h_direct, violations)

// AFTER (2 params)
escalate_contract_parent(tree: &mut VTreeMutContext, ctx: &EscalationContext)
```

**New Types Needed:**
- `EscalationContext` – groups multiple node IDs + flags
- `VTreeMutContext<V>` – vnodes + violations together

**Files:** `src/graph/algorithm/rebalance/resolve.rs`

---

### 🟠 Pattern 3: Mutable Collections Should Be Methods
**Severity:** HIGH | **Locations:** 15+ | **Effort:** 2-3 days

Free functions mutating `&mut Vec`:

```rust
// BEFORE: push_contraction_child_violations(vnodes, p, heaviest, violations)
// AFTER: violations.push_contraction_child(vnodes, p, heaviest)
```

**New Type:**
- `ViolationQueue` – encapsulates violation vector operations

**Affected Functions:**
- `push_contraction_child_violations()`
- `push_side_effect_violations()`
- `push_promoted_violations()`
- Similar patterns × 15+

**Files:** `src/graph/algorithm/violation_push.rs`

---

### 🟠 Pattern 4: Coordinate Range Pairs → Single Type
**Severity:** HIGH | **Locations:** 20+ | **Effort:** 1-2 days

`(query_lo, query_hi)` always passed together:

```rust
// BEFORE: decompose_basis(gid, query_lo, query_hi, basis)
// AFTER: decompose_basis(gid, range: CoordinateRange<C>, basis)

// Same for:
range_sum_inner(gid, query_lo, query_hi)  → range_sum_inner(gid, range)
```

**New Type:**
- `CoordinateRange<C>` – wraps `(lo: C, hi: C)`

**Files:** `src/graph/algorithm/query/`, `src/graph/algorithm/plateau/`

---

### 🟡 Pattern 5: Plateau Context – Reduce Parameter Repetition
**Severity:** MEDIUM | **Locations:** 10+ | **Effort:** 2 days

Every plateau method takes `&Arena<GNode<C, V>>`:

```rust
impl DynamicPlateauTracker {
    on_observe_impl(&mut self, gnodes: &Arena, g_id, value)
    on_bootstrap_split_impl(&mut self, gnodes: &Arena, g_id)
    on_catalytic_split_impl(&mut self, gnodes: &Arena, g_id, left_id)
    // ... 10+ more methods all with same gnodes param
}
```

**Access Pattern:** All called via GvGraph wrappers that have gnodes available.

**Solution:** Introduce `PlateauTrackerWithContext` or similar wrapper to provide gnodes implicitly.

**Files:** `src/graph/algorithm/plateau/dynamic_tracker/`

---

## Secondary Patterns (Not Critical)

### 🟡 Pattern 6: Eviction Context Bundling
**Severity:** MEDIUM | **Locations:** 1-2 | **Effort:** 0.5 day

```rust
// BEFORE: on_evict(gnodes, gnode_id, parent_id, parent_state, parent_lo, parent_hi)
// AFTER: on_evict(gnodes, ctx: EvictionContext<C>)

pub struct EvictionContext<C> {
    evicted_id: GNodeId,
    parent_id: GNodeId,
    parent_state: GState,
    parent_range: (C, C),
}
```

**Files:** `src/graph/algorithm/evict.rs`

---

### 🟢 Pattern 7: Simple Wrappers
**Severity:** LOW | **Locations:** 5+ | **Effort:** Refactor with other changes

```rust
// OPTIONAL: Inline or find semantic home
fn structural_child_count(vnodes, id) → vnodes.get(id).child_count()
fn any_child_violated(vnodes, node) → ...
```

---

## Implementation Roadmap

### Phase 1: Tell-Don't-Ask (1 day)
- [ ] `bootstrap_split()` → `GvGraph::bootstrap_split()`
- [ ] `catalytic_split()` → `GvGraph::catalytic_split()`

### Phase 2: Context Structures (3 days)
- [ ] Create `EscalationContext` struct
- [ ] Create `VTreeMutContext<V>` struct
- [ ] Refactor escalation functions to use contexts
- [ ] Create `CoordinateRange<C>` struct
- [ ] Update query functions to use ranges

### Phase 3: Collection Ownership (2 days)
- [ ] Create `ViolationQueue` type
- [ ] Convert violation pushing functions to methods
- [ ] Create plateau element collector

### Phase 4: Implicit Parameters (2 days)
- [ ] Reduce plateau method signatures (gnodes parameter)
- [ ] Bundle eviction parameters
- [ ] Clean up simple wrappers

---

## Quick Parameter Reduction Checklist

### Functions to Convert to Methods

| Free Function | Should Be Method On | Param Reduction |
|--------------|-------------------|-----------------|
| `bootstrap_split(graph, id)` | `GvGraph` | 1 → 0 |
| `catalytic_split(graph, id)` | `GvGraph` | 1 → 0 |
| `propagate_v_sums(vnodes, id)` | `VTree` | 1 → 0 |
| `push_*_violations(vnodes, ..., violations)` | `ViolationQueue` | 2-3 → 1 |

### Structures to Introduce

| New Type | Bundles | Elimination Count |
|----------|---------|------------------|
| `EscalationContext` | `p, heaviest, merged, h_direct` | 4 → 1 param |
| `CoordinateRange<C>` | `lo, hi` | 2 → 1 param (×20+ locations) |
| `ViolationQueue` | `vnodes, violations` | 2 → 1 param |
| `BasisElementInfo<C>` | `gnode, key, lo, hi, depth` | 5 → 1 param |
| `EvictionContext<C>` | `gnode_id, parent_id, state, lo, hi` | 5 → 1 param |

---

## Estimated Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Avg function params (hotspots) | 5.2 | 2.1 | -60% |
| Free functions taking `&mut` | 25+ | 10 | -60% |
| "Tell-Don't-Ask" violations | 5 | 0 | 100% |
| Total lines of parameter passing | ~1500 | ~600 | -60% |

---

## Dependencies

**Phase 1 → 2:** Split methods needed before context refactoring
**Phase 2 → 3:** Context types simplify violation queue
**Phase 3 → 4:** Everything prepared for final cleanup

