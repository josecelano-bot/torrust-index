# Refactoring Patterns – Implementation Roadmap (Updated)

This roadmap reflects the current repository state, not the original pre-refactor projection.

## Completed Phases

### Phase A: Tell-Don't-Ask split ownership

- Completed in `src/graph/algorithm/split.rs`.
- `bootstrap_split` and `catalytic_split` are implemented as `GvGraph` methods.

### Phase B: Escalation context grouping

- Completed in `src/graph/algorithm/rebalance/context.rs` and `src/graph/algorithm/rebalance/resolve.rs`.
- `EscalationContext` and `VTreeMutContext` are active in escalation/resolve orchestration.

### Phase C: CoordinateRange rollout

- Completed in query path and shared spatial utilities.
- `CoordinateRange<C>` is in `src/spatial/range.rs` and used by contour/range sum query internals.

## Active Phase

### Phase D: ViolationQueue migration (in progress)

Objective: remove direct `&mut Vec<VNodeId>` plumbing from high-churn orchestration sites first, while preserving behavior.

#### D1. Introduce thin queue facade

- Add `ViolationQueue` wrapper around existing vector operations.
- Keep compatibility with existing free-function push helpers.

Success criteria:

- New type compiles without changing behavior.
- Existing call sites can migrate incrementally.

#### D2. Migrate high-churn call paths

Priority targets:

1. `src/graph/algorithm/rebalance/resolve.rs`
2. `src/graph/algorithm/split/helpers.rs`

Success criteria:

- Behavior-preserving migration to queue methods.
- Targeted tests and full test suite pass.

#### D3. Migrate remaining queue call sites

Secondary targets:

- `src/graph/algorithm/evict.rs`
- Additional rebalance/violation call chains.

Success criteria:

- No direct mutation call sites remain in orchestration logic.
- Push helper internals remain centralized.

## Optional Phase

### Phase E: Plateau context flattening (deferred)

This remains optional. The recent helper-phase extraction in dynamic tracker already reduced hotspot complexity significantly, so further signature flattening is only justified if new metrics identify clear ROI.

## Validation Gates Per Slice

1. `cargo check --all-features`
2. Targeted tests for touched module
3. `cargo test --all-features`

## Exit Criteria

- Pattern 4 complete (queue ownership in primary orchestration paths).
- No behavior regressions in violation propagation.
- Documentation synchronized after each slice.
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

