# Refactoring Patterns – Detailed Execution Checklist

Step-by-step checklist for implementing each refactoring pattern.

---

## Pattern 1: Tell-Don't-Ask – Split Operations → Methods

### Objective
Convert `bootstrap_split(graph, id)` and `catalytic_split(graph, id)` to GvGraph methods.

### Current State
- **Files:** `src/graph/algorithm/split.rs` (free functions line ~70)
- **Calls:** ~20+ call sites in GvGraph implementation
- **Status:** Decoupled, but violates "tell don't ask"

### Step 1.1: Create GvGraph Methods
**Acceptance Criteria:**
- [ ] Methods added to GvGraph impl block
- [ ] Signatures: `pub(crate) fn bootstrap_split(&mut self, g_id: GNodeId)`
- [ ] Both methods call through to original impl (initially)

**Code Change:**
```rust
impl<C, V, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn bootstrap_split(&mut self, g_id: GNodeId) {
        bootstrap_split(self, g_id);  // Delegate to free function
    }
    
    pub(crate) fn catalytic_split(&mut self, g_id: GNodeId) {
        catalytic_split(self, g_id);  // Delegate to free function
    }
}
```

**Testing:**
```bash
cargo test --lib graph::algorithm::split
cargo test --lib graph::attempt_split  # Full workflow
```

### Step 1.2: Update Call Sites
**Current Pattern:**
```rust
// In GvGraph methods
bootstrap_split(self, g_id);
catalytic_split(self, g_id);
```

**New Pattern:**
```rust
// In GvGraph methods
self.bootstrap_split(g_id);
self.catalytic_split(g_id);
```

**Automated Search & Replace:**
```bash
# Find all call sites
grep -n "bootstrap_split(self," src/graph/**/*.rs
grep -n "catalytic_split(self," src/graph/**/*.rs

# Count before: should be ~10-15
# After: same count, different pattern
```

**Acceptance Criteria:**
- [ ] All `bootstrap_split(self, ...)` → `self.bootstrap_split(...)`
- [ ] All `catalytic_split(self, ...)` → `self.catalytic_split(...)`
- [ ] Tests pass: `cargo test --lib split`

### Step 1.3: Mark Free Functions Private (Optional)
**Current:** Public functions in split.rs
**Target:** Mark private or move to impl

```rust
// BEFORE
pub(crate) fn bootstrap_split(graph: &mut GvGraph, g_id: GNodeId) { ... }
pub(crate) fn catalytic_split(graph: &mut GvGraph, g_id: GNodeId) { ... }

// AFTER
fn bootstrap_split(graph: &mut GvGraph, g_id: GNodeId) { ... }
fn catalytic_split(graph: &mut GvGraph, g_id: GNodeId) { ... }
```

**Acceptance Criteria:**
- [ ] Functions marked private (no `pub`)
- [ ] All tests still pass
- [ ] No external calls to free functions

### Gate: Code Review Checkpoint
**Testing:**
```bash
cargo test --lib split
cargo test --lib graph::
cargo test
```

**Validation:**
- [ ] No regressions in split tests
- [ ] No regressions in full test suite
- [ ] Complexity metrics stable in metrics-output/

---

## Pattern 2: Context Structures – Escalation Operations

### Objective
Create `EscalationContext` to reduce function parameters from 5-7 to 2.

### Current State
- **Files:** `src/graph/algorithm/rebalance/resolve.rs` (line ~30-100)
- **Functions:** 3 functions with parameter bloat
- **Total parameters:** 18 params across 3 functions

### Step 2.1: Define Context Types
**Location:** Add to `src/graph/algorithm/rebalance/mod.rs` or new file `context.rs`

```rust
/// Context for node position during escalation.
pub struct EscalationNodeContext {
    pub parent_id: VNodeId,
    pub heaviest_id: VNodeId,
    pub merged_id: Option<VNodeId>,
    pub heaviest_is_direct_child: bool,
}

/// Mutable references needed for tree rebalancing.
pub struct VTreeMutContext<'a, V: Accumulator> {
    pub vnodes: &'a mut Arena<VNode<V>>,
    pub violations: &'a mut Vec<VNodeId>,
}
```

**Acceptance Criteria:**
- [ ] Struct defined with proper documentation
- [ ] Derives Debug, Clone (where appropriate)
- [ ] Uses semantic field names (not `x`, `y`, `z`)

### Step 2.2: Create Helper Constructor
**Add:**
```rust
impl EscalationNodeContext {
    pub fn new(parent_id: VNodeId, heaviest_id: VNodeId, h_direct: bool) -> Self {
        Self {
            parent_id,
            heaviest_id,
            merged_id: None,
            heaviest_is_direct_child: h_direct,
        }
    }
}
```

### Step 2.3: Refactor Function Signatures

**BEFORE:**
```rust
fn escalate_contract_parent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    p: VNodeId,
    heaviest: VNodeId,
    h_direct: bool,
    violations: &mut Vec<VNodeId>,
) -> Option<VNodeId>
```

**AFTER:**
```rust
fn escalate_contract_parent<V: Accumulator>(
    tree: &mut VTreeMutContext<V>,
    ctx: &mut EscalationNodeContext,
) -> Option<VNodeId>
```

**Implementation Notes:**
- Access `vnodes` as `tree.vnodes`
- Access `violations` as `tree.violations`
- Access node IDs as `ctx.parent_id`, `ctx.heaviest_id`
- Return value: update `ctx.merged_id` before returning

**Acceptance Criteria:**
- [ ] All three escalation functions refactored
- [ ] Parameter count: 5-7 → 2 for each
- [ ] Documentation updated
- [ ] Tests pass: `cargo test --lib rebalance`

### Step 2.4: Update Call Sites

**Pattern:** All calls in `resolve.rs`

**BEFORE:**
```rust
let merged = escalate_contract_parent(vnodes, p_id, heaviest, h_direct, violations)?;
```

**AFTER:**
```rust
let mut tree = VTreeMutContext { vnodes, violations };
let mut ctx = EscalationNodeContext::new(p_id, heaviest, h_direct);
let merged = escalate_contract_parent(&mut tree, &mut ctx)?;
```

**Acceptance Criteria:**
- [ ] All 5-10 call sites updated
- [ ] Variable names reflect context purpose
- [ ] Tests pass

### Gate: Complexity Analysis
**Before Refactoring:**
```bash
rust-code-analysis-cli -o metrics-output/ src/graph/algorithm/rebalance/resolve.rs
# Note CC count for escalate_* functions
```

**After Refactoring:**
```bash
rust-code-analysis-cli -o metrics-output/ src/graph/algorithm/rebalance/resolve.rs
# Expect: CC same (logic unchanged), but Readability improved (fewer params)
```

**Validation:**
- [ ] Cyclomatic complexity unchanged
- [ ] Function signature length reduced by 60%

---

## Pattern 3: Coordinate Range → New Type

### Objective
Replace `(query_lo: C, query_hi: C)` pairs with `range: CoordinateRange<C>`.

### Current State
- **Files:** `src/graph/algorithm/query/`, `src/spatial/`
- **Occurrences:** 20+ locations
- **Impact:** Low-complexity refactoring, high readability gain

### Step 3.1: Define Type
**Location:** `src/spatial/range.rs` (new file)

```rust
/// Coordinate interval for queries and decomposition.
#[derive(Debug, Clone, Copy)]
pub struct CoordinateRange<C: Coordinate> {
    pub lo: C,
    pub hi: C,
}

impl<C: Coordinate> CoordinateRange<C> {
    pub fn new(lo: C, hi: C) -> Self {
        Self { lo, hi }
    }
    
    pub fn is_empty(&self) -> bool {
        self.lo >= self.hi
    }
    
    pub fn clamp(self, domain_lo: C, domain_hi: C) -> Self {
        let lo = if self.lo < domain_lo { domain_lo } else { self.lo };
        let hi = if self.hi > domain_hi { domain_hi } else { self.hi };
        Self { lo, hi }
    }
}
```

**Acceptance Criteria:**
- [ ] Type defined with standard methods
- [ ] Derives: Debug, Clone, Copy
- [ ] Helper methods: `is_empty()`, `clamp()`

### Step 3.2: Update Function Signatures

**Files to Update:**
- `src/graph/algorithm/query/contour.rs` – `decompose_basis()`
- `src/graph/algorithm/query/range_sum.rs` – `range_sum_inner()`
- `src/graph/algorithm/plateau/` – multiple query functions

**BEFORE:**
```rust
fn decompose_basis(
    &self,
    gid: GNodeId,
    query_lo: C,
    query_hi: C,
    basis: &mut Vec<BasisElement<C, V>>,
)
```

**AFTER:**
```rust
fn decompose_basis(
    &self,
    gid: GNodeId,
    range: CoordinateRange<C>,
    basis: &mut Vec<BasisElement<C, V>>,
)
```

**Acceptance Criteria:**
- [ ] All 20+ function signatures updated
- [ ] Tests pass

### Step 3.3: Update Call Sites

**Pattern:** Replace pair construction with type

**BEFORE:**
```rust
self.range_sum_inner(gid, query_lo, query_hi)
self.decompose_basis(gid_from_left, query_lo, query_hi, basis)
```

**AFTER:**
```rust
self.range_sum_inner(gid, CoordinateRange::new(query_lo, query_hi))
self.decompose_basis(
    gid_from_left,
    CoordinateRange { lo: query_lo, hi: query_hi },
    basis
)
```

**Acceptance Criteria:**
- [ ] All 20+ call sites updated
- [ ] Type inference succeeds without explicit `<C>`
- [ ] Tests pass: `cargo test --lib query`

### Step 3.4: Verification

**Metrics Check:**
```bash
# Before
grep -r "query_lo.*query_hi" src/graph/algorithm/ | wc -l  # ~40 occurrences
grep -r "query_lo: C" src/graph/algorithm/ | wc -l         # ~10 params

# After
grep -r "CoordinateRange" src/graph/algorithm/ | wc -l     # ~40 occurrences
grep -r "query_lo: C" src/graph/algorithm/ | wc -l         # ~0 params
```

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

