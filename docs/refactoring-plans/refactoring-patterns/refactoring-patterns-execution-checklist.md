# Refactoring Patterns – Execution Checklist (Current)

This checklist tracks remaining pattern work from the current code state.

## Completed Items (Do Not Re-Implement)

- [x] Pattern 1: split tell-don't-ask migration
- [x] Pattern 2: escalation context grouping
- [x] Pattern 3: `CoordinateRange<C>` adoption in query path

## Active Checklist: Pattern 4 (ViolationQueue)

### Step 4.1: Introduce queue facade

- [ ] Add `ViolationQueue` wrapper around `Vec<VNodeId>` in violation-push layer.
- [ ] Provide method aliases for key push operations.
- [ ] Keep existing free functions for compatibility.

Acceptance:

- [ ] Compiles without behavior change.

### Step 4.2: Migrate rebalance resolve path

- [ ] Migrate `src/graph/algorithm/rebalance/resolve.rs` call sites to queue methods.
- [ ] Keep orchestration readable; avoid signature churn outside this module.

Acceptance:

- [ ] `cargo test --all-features` passes.
- [ ] No residual-violation regressions in debug audits.

### Step 4.3: Migrate split helper path

- [ ] Migrate `src/graph/algorithm/split/helpers.rs` queue call sites.

Acceptance:

- [ ] Split-specific tests continue to pass.

### Step 4.4: Migrate secondary call sites

- [ ] Evaluate and migrate `src/graph/algorithm/evict.rs` and any remaining algorithm callers.

Acceptance:

- [ ] No high-churn call paths still pass raw queue mutation vectors.

## Deferred Checklist: Pattern 5 (Plateau Context)

- [ ] Re-open only if updated complexity metrics show clear ROI.
- [ ] If re-opened, run as separate initiative with fresh baseline.

## Validation Gates (Per Slice)

- [ ] `cargo check --all-features`
- [ ] Targeted tests for touched modules
- [ ] `cargo test --all-features`

## Stop Conditions

- [ ] Stop if violation order semantics change.
- [ ] Stop if any slice causes unresolved violations after rebalance.
- [ ] Stop if complexity regresses in stabilized dynamic-tracker modules.

**Acceptance Criteria:**
- [ ] All `query_lo/query_hi` pairs → `CoordinateRange`
- [ ] Readability: method signatures 20% shorter average
- [ ] Tests pass

---

## Pattern 4: Violation Queue → Type

### Objective
Encapsulate `&mut Vec<VNodeId>` into a `ViolationQueue` type.

### Current State
- **Files:** `src/graph/algorithm/violation_push.rs`
- **Functions:** ~15 functions accepting `&mut Vec<VNodeId>`
- **Scope:** Phase 3 (reduces accidental complexity)

### Step 4.1: Define Type
**Location:** `src/graph/algorithm/rebalance/violation_queue.rs` (new file)

```rust
pub struct ViolationQueue {
    queue: Vec<VNodeId>,
}

impl ViolationQueue {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }
    
    pub fn push(&mut self, id: VNodeId) {
        self.queue.push(id);
    }
    
    pub fn next(&mut self) -> Option<VNodeId> {
        if self.queue.is_empty() { None } else { Some(self.queue.remove(0)) }
    }
    
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
    
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
```

**Acceptance Criteria:**
- [ ] Implements core queue operations
- [ ] Encapsulates Vec internals

### Step 4.2: Migrate Functions

Move violation-pushing functions to methods:

**BEFORE:**
```rust
pub fn push_contraction_child_violations<V: Accumulator>(
    vnodes: &Arena<VNode<V>>,
    p: VNodeId,
    heaviest: VNodeId,
    violations: &mut Vec<VNodeId>,
) { ... }
```

**AFTER:**
```rust
impl ViolationQueue {
    pub fn push_contraction_child<V: Accumulator>(
        &mut self,
        vnodes: &Arena<VNode<V>>,
        p: VNodeId,
        heaviest: VNodeId,
    ) { ... }
}
```

**Acceptance Criteria:**
- [ ] All 15 violation functions → queue methods
- [ ] No `&mut Vec` parameters remain
- [ ] Tests pass

### Gate: Test All Patterns Together
**Integrated Test:**
```bash
cargo test --lib rebalance
cargo test --lib graph::algorithm
cargo test  # Full suite
```

**Acceptance:**
- [ ] All tests pass
- [ ] No new clippy warnings
- [ ] Cargo check succeeds

---

## Summary: Checklist Status Template

Use this template to track progress:

```
## Pattern 1: Tell-Don't-Ask ✅
- [x] Step 1.1: Create methods
- [x] Step 1.2: Update call sites
- [x] Step 1.3: Mark private
- [x] Gate: Tests pass

## Pattern 2: Context Structures 🟡
- [x] Step 2.1: Define types
- [x] Step 2.2: Constructors
- [ ] Step 2.3: Refactor signatures (IN PROGRESS)
- [ ] Step 2.4: Update calls
- [ ] Gate: Complexity analysis

## Pattern 3: Coordinate Range ⏳
- [ ] Step 3.1: Define type
- [ ] Step 3.2: Update signatures
- [ ] Step 3.3: Update calls
- [ ] Step 3.4: Verification

## Pattern 4: Violation Queue ⏳
- [ ] Step 4.1: Define type
- [ ] Step 4.2: Migrate functions
```

