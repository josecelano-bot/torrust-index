# Refactoring Patterns – Implementation Roadmap

Cohesive strategy for applying all patterns across the codebase systematically.

---

## Phase 1: Foundation & Preparation (Week 1-2)

### 1.1 Establish Testing Baseline

```bash
# Step 1: Run full test suite
cargo test --lib --all 2>&1 | tee test-baseline.log

# Step 2: Measure current coverage
cargo tarpaulin --lib --timeout 300 --out Html

# Step 3: Benchmark key functions (if applicable)
cargo bench --bench depth 2>&1 | tee bench-baseline.log

# Step 4: Document metrics
cargo expand --lib 2>&1 | grep -E "fn " | wc -l > metrics-baseline.txt
```

**Success Criteria:**
- ✓ All tests pass
- ✓ Coverage baseline > 90%
- ✓ Benches complete without regression

### 1.2 Create Feature Branch & Documentation

```bash
# Clone current state as "before" reference
git checkout -b refactor/parameter-complexity-reduction
git tag "baseline/parameter-reduction-start"

# Log baseline metrics in branch
echo "Baseline: $(date)" >> REFACTORING_LOG.md
```

### 1.3 Analysis Phase: Map Current Patterns

**Create a physical map of violations:**

```rust
// src/graph/algorithm/REFACTORING_MAP.md (Generated document)

## Tell-Don't-Ask Violations Found:

### Free Functions Taking &mut GvGraph:
- bootstrap_split(graph: &mut GvGraph, gid: GVNodeId)
  Location: src/graph/algorithm/split.rs:42
  Call sites: 3
  Status: PRIORITY HIGH

- catalytic_split(graph: &mut GvGraph, gid: GVNodeId, ...)
  Location: src/graph/algorithm/split.rs:89
  Call sites: 5
  Status: PRIORITY HIGH

### Free Functions Taking &mut Arena<GNode>:
- [Similar enumeration for Arena parameters]

### Parameter Pairs Found:
- (query_lo, query_hi): ~40 occurrences
- (parent_id, child_id): ~25 occurrences
- (vnodes, violations): ~30 occurrences
```

---

## Phase 2: Pattern 1 – Tell-Don't-Ask (Week 2-3)

### 2.1 Convert `bootstrap_split`

**Step 1: Add method to GvGraph**
```rust
// src/graph/mod.rs

impl GvGraph {
    /// Bootstrap split operation on graph structure
    pub fn bootstrap_split(&mut self, gid: GVNodeId) {
        // Move body from free function here
        // self replaces graph parameter
    }
}
```

**Step 2: Keep free function as wrapper (temporary)**
```rust
// src/graph/algorithm/split.rs - DEPRECATED

#[deprecated(note = "Use GvGraph::bootstrap_split() instead")]
pub fn bootstrap_split(graph: &mut GvGraph, gid: GVNodeId) {
    graph.bootstrap_split(gid)
}
```

**Step 3: Update call sites**
```bash
# Search all call sites
grep -r "bootstrap_split(" src/ tests/

# Replace in each file:
# BEFORE: bootstrap_split(self, g_id)
# AFTER: self.bootstrap_split(g_id)
```

**Step 4: Verify & Commit**
```bash
cargo test --lib split

# Once confirmed working:
git commit -m "refactor(graph): Move bootstrap_split to GvGraph method

- Closer to data ownership
- Clearer intent (self.operation)
- 3 call sites updated
- Deprecated wrapper for compatibility"
```

### 2.2 Repeat for `catalytic_split`

Same pattern as 2.1, but prioritize based on:
1. How many call sites exist
2. How tightly coupled to GvGraph data
3. Test coverage of the function

### 2.3 Phase 2 Success Criteria

```
✓ bootstrap_split: Free function → GvGraph method
✓ catalytic_split: Free function → GvGraph method
✓ All tests pass: cargo test --lib split
✓ Call sites: Same count, cleaner syntax
✓ No compiler warnings
✓ Branch: docs/refactoring-phase-1-tell-dont-ask
```

---

## Phase 3: Pattern 2 – Escalation Context (Week 3-4)

### 3.1 Create EscalationContext Structure

**File: src/graph/algorithm/rebalance/context.rs (NEW)**
```rust
/// Context for escalation operations
pub struct EscalationContext {
    pub violations: Vec<VNodeId>,
    pub heaviest_id: Option<VNodeId>,
    pub heaviest_depth: i32,
}

impl EscalationContext {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            heaviest_id: None,
            heaviest_depth: i32::MAX,
        }
    }
    
    pub fn add_violation(&mut self, vnid: VNodeId) {
        self.violations.push(vnid);
    }
}
```

### 3.2 Refactor `escalate_contract_parent`

**BEFORE:**
```rust
fn escalate_contract_parent(
    vnodes: &[-mut Vec<VNode>],
    p: ParentLink,
    heaviest: VNodeId,
    h_direct: i32,
    violations: &mut Vec<VNodeId>,
) -> bool { ... }  // 5 params
```

**AFTER:**
```rust
fn escalate_contract_parent(
    vnodes: &mut Vec<VNode>,
    p: ParentLink,
    context: &mut EscalationContext,
) -> bool { ... }  // 3 params

// Usage:
escalate_contract_parent(vnodes, parent, &mut ctx)
```

### 3.3 Refactor `escalate_try_contract_grandparent`

Same pattern as 3.2, reducing from 7 params to 3.

### 3.4 Update Test File

```rust
// tests/integration.rs
#[test]
fn test_escalation_with_context() {
    let mut context = EscalationContext::new();
    
    // Cleaner test setup - fewer params to track
    escalate_contract_parent(
        &mut vnodes,
        parent_link,
        &mut context,
    )
}
```

### 3.5 Phase 3 Success Criteria

```
✓ EscalationContext struct created
✓ escalate_contract_parent: 5 → 3 params
✓ escalate_try_contract_grandparent: 7 → 3 params
✓ escalate_skip_promote: 5 → 3 params
✓ All tests pass: cargo test --lib rebalance
✓ Parameter reduction measurable: ~40%
✓ Branch: docs/refactoring-phase-2-escalation-context
```

---

## Phase 4: Pattern 3 – Coordinate Range (Week 4-5)

### 4.1 Create CoordinateRange Type

**File: src/tree/coordinate/range.rs (ENHANCEMENT)**
```rust
/// Represents a closed interval [lo, hi] in coordinate space
#[derive(Debug, Clone, Copy)]
pub struct CoordinateRange {
    lo: i32,
    hi: i32,
}

impl CoordinateRange {
    pub fn new(lo: i32, hi: i32) -> Self {
        assert!(lo <= hi, "Invalid range");
        Self { lo, hi }
    }
    
    pub fn lo(&self) -> i32 { self.lo }
    pub fn hi(&self) -> i32 { self.hi }
    pub fn span(&self) -> i32 { self.hi - self.lo + 1 }
}
```

### 4.2 Update Function Signatures in Query Path

**Location: src/graph/algorithm/search/query.rs**

**BEFORE:**
```rust
fn decompose_basis(
    gid: GVNodeId,
    query_lo: i32,
    query_hi: i32,
    basis: &mut Vec<GNode>,
) -> bool { ... }  // 4 params
```

**AFTER:**
```rust
fn decompose_basis(
    gid: GVNodeId,
    range: CoordinateRange,
    basis: &mut Vec<GNode>,
) -> bool { ... }  // 3 params
```

### 4.3 Update All Call Sites

```bash
# Search for query_lo/query_hi patterns
grep -r "query_lo.*query_hi" src/graph/algorithm/search/

# Replace patterns:
# BEFORE: decompose_basis(node_id, 100, 200, basis)
# AFTER: decompose_basis(node_id, CoordinateRange::new(100, 200), basis)

# Or use helper:
let range = CoordinateRange::from_bounds(query_lo, query_hi);
decompose_basis(node_id, range, basis)
```

### 4.4 Update range_sum_inner

Similar pattern, consolidate coordinate parameters into CoordinateRange.

### 4.5 Phase 4 Success Criteria

```
✓ CoordinateRange type created
✓ 40+ lo/hi pair occurrences replaced
✓ 3+ functions updated with new signature
✓ All tests pass: cargo test --lib query
✓ Type safety improved (can't pass mismatched lo/hi)
✓ Branch: docs/refactoring-phase-3-coordinate-range
```

---

## Phase 5: Pattern 4 – Violation Queue (Week 5-6)

### 5.1 Create ViolationQueue

**File: src/graph/algorithm/rebalance/queue.rs (NEW)**
```rust
pub struct ViolationQueue {
    violations: Vec<VNodeId>,
}

impl ViolationQueue {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }
    
    pub fn push_contraction(&mut self, vnid: VNodeId) {
        self.violations.push(vnid);
    }
    
    pub fn push_side_effects(&mut self, vnid: VNodeId) {
        self.violations.push(vnid);
    }
    
    pub fn push_promoted(&mut self, vnid: VNodeId) {
        self.violations.push(vnid);
    }
    
    pub fn drain(&mut self) -> impl Iterator<Item = VNodeId> {
        self.violations.drain(..)
    }
}
```

### 5.2 Consolidate Violation Functions

**Before:**
```rust
fn push_contraction_child_violations(
    vnodes: &[-mut Vec<VNode>],
    p: ParentLink,
    heaviest: VNodeId,
    violations: &mut Vec<VNodeId>,
)

fn push_side_effect_violations(
    vnodes: &[-mut Vec<VNode>],
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
)
```

**After:**
```rust
// All consolidated into ViolationQueue methods
queue.push_contraction(p, heaviest)
queue.push_side_effects(node)
```

### 5.3 Integrate with EscalationContext

```rust
pub struct EscalationContext {
    pub violations: ViolationQueue,  // Changed from Vec
    pub heaviest_id: Option<VNodeId>,
    pub heaviest_depth: i32,
}

impl EscalationContext {
    pub fn add_violation(&mut self, vnid: VNodeId) {
        self.violations.push_contraction(vnid);
    }
}
```

### 5.4 Update Rebalance Logic

Replace all violation queue management with structured approach.

### 5.5 Phase 5 Success Criteria

```
✓ ViolationQueue struct created
✓ 15+ violation functions consolidated
✓ 3-4 params reduced to 2 params per call
✓ All tests pass: cargo test --lib rebalance
✓ Violation handling centralized & tested
✓ Branch: docs/refactoring-phase-4-violation-queue
```

---

## Phase 6: Integration & Cleanup (Week 6-7)

### 6.1 End-to-End Test

Create comprehensive integration test combining all patterns:

```rust
#[test]
fn test_refactored_algorithm_end_to_end() {
    let mut graph = GvGraph::new();
    let mut context = EscalationContext::new();
    
    // All patterns working together
    let range = CoordinateRange::new(100, 200);
    graph.decompose_basis(root_id, range, &mut basis)?;
    
    escalate_contract_parent(
        &mut vnodes,
        parent,
        &mut context,
    )?;
    
    for violation_id in context.violations.drain() {
        graph.process_violation(violation_id);
    }
}
```

### 6.2 Remove Deprecated Functions

```bash
# Remove @deprecated wrappers once confident
git grep -l "@deprecated\|#\[deprecated"
# Delete those functions
```

### 6.3 Performance Verification

```bash
# Ensure no regressions
cargo bench --bench depth
```

### 6.4 Documentation Updates

Update main README and architecture docs.

### 6.5 Phase 6 Success Criteria

```
✓ Integration tests pass
✓ Benchmark baseline maintained
✓ No compiler warnings
✓ Documentation updated
✓ Code review checklist completed
✓ Final PR ready for merge
```

---

## Implementation Checklist by Phase

### Phase 1: Foundation
- [ ] Test baseline established
- [ ] Coverage measured
- [ ] Branch created
- [ ] Refactoring map documented

### Phase 2: Tell-Don't-Ask
- [ ] bootstrap_split moved to GvGraph
- [ ] catalytic_split moved to GvGraph
- [ ] Call sites updated
- [ ] Tests pass
- [ ] PR: `refactor/tell-dont-ask`

### Phase 3: Escalation Context
- [ ] EscalationContext struct created
- [ ] escalate_contract_parent refactored (5→3 params)
- [ ] escalate_try_contract_grandparent refactored (7→3 params)
- [ ] escalate_skip_promote refactored (5→3 params)
- [ ] Tests pass
- [ ] PR: `refactor/escalation-context`

### Phase 4: Coordinate Range
- [ ] CoordinateRange type created
- [ ] decompose_basis updated
- [ ] range_sum_inner updated
- [ ] All call sites (40+) replaced
- [ ] Tests pass
- [ ] PR: `refactor/coordinate-range`

### Phase 5: Violation Queue
- [ ] ViolationQueue struct created
- [ ] Violation functions consolidated
- [ ] Integration with EscalationContext
- [ ] 15+ functions replaced
- [ ] Tests pass
- [ ] PR: `refactor/violation-queue`

### Phase 6: Integration & Cleanup
- [ ] End-to-end tests
- [ ] Deprecated functions removed
- [ ] Performance verified
- [ ] Documentation updated
- [ ] PR: `refactor/final-integration`

---

## Risk Mitigation

### Risk 1: Regression in Rebalance Logic
**Mitigation:**
- Run full test suite after each phase
- Keep deprecated wrappers for 1 full test cycle
- Create property-based tests for invariants

### Risk 2: Performance Regression
**Mitigation:**
- Benchmark before & after each phase
- Keep original structures if measurable overhead
- Profile hot paths

### Risk 3: Merge Conflicts
**Mitigation:**
- Keep phases independent
- Merge to main after each phase if possible
- Coordinate with team on branching strategy

### Risk 4: Incomplete Refactoring
**Mitigation:**
- Strict checklist for each phase
- Code review after every phase
- Test coverage must stay >= 95%

---

## Rollback Strategy

If refactoring introduces issues:

```bash
# Quick rollback to baseline
git reset --hard baseline/parameter-reduction-start

# Or rollback one phase
git revert <commit-hash>

# Identify issue, create fix branch
git checkout -b refactor-fix/<issue>
```

---

## Success Metrics After Complete Refactoring

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Avg function params (hotspots) | 4.2 | ~2.5 | 2.5 |
| Tell-Don't-Ask violations | 2 | 0 | 0 |
| Parameter pair occurrences | 40+ | ~0 | 0 |
| avg function complexity | Stable | Stable | Stable |
| Test coverage | >95% | >95% | >95% |
| Build time | Baseline | ±5% | ±5% |
| Runtime performance | Baseline | ±2% | ±2% |

---

## Post-Refactoring Maintenance Guide

### What Should Change:
- Function signatures (fewer, grouped parameters)
- Call sites (cleaner, less boilerplate)
- Type safety (CoordinateRange prevents invalid ranges)
- Code organization (Tell-Don't-Ask clearer ownership)

### What Should Stay the Same:
- Algorithm logic (same computations)
- Performance characteristics
- Test coverage percentage
- Public API (if external consumers exist)

### Ongoing Best Practices:
1. **No more 5+ parameter functions** - use context pattern
2. **Group related parameters** - use structs/types
3. **Tell don't ask** - put operations close to data
4. **Consolidate mutation** - centralize &mut refs

