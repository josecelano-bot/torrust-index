# Refactoring Patterns Analysis (Current)

This analysis captures what is already solved and where accidental complexity remains.

## Completed Pattern Outcomes

### Pattern 1: Tell-Don't-Ask for split operations

Result:

- Split behavior is owned by `GvGraph` methods.
- Call sites use `self.bootstrap_split(...)` and `self.catalytic_split(...)`.

Primary evidence:

- `src/graph/algorithm/split.rs`

Why this matters:

- Split orchestration now lives where graph ownership already exists.
- Parameter passing overhead for split entry points is removed.

### Pattern 2: Context grouping for escalation path

Result:

- Escalation helper flow uses explicit context structs.
- Multi-parameter orchestration signatures are replaced by named contexts.

Primary evidence:

- `src/graph/algorithm/rebalance/context.rs`
- `src/graph/algorithm/rebalance/resolve.rs`

Why this matters:

- Function intent is clearer.
- State transitions are easier to trace across escalation phases.

### Pattern 3: Coordinate range abstraction

Result:

- Query internals use `CoordinateRange<C>` in recursive paths.
- Loose lo/hi parameter pairing in core query recursion is eliminated.

Primary evidence:

- `src/spatial/range.rs`
- `src/graph/algorithm/query/range_sum.rs`
- `src/graph/algorithm/query/contour.rs`

Why this matters:

- Range semantics are explicit and reusable.
- Range invariants are centralized on one type.

## Remaining High-Value Gap

### Pattern 4: Violation queue ownership is still split

Current state:

- Violation push logic is well centralized in `violation_push.rs`.
- Orchestration still passes raw `&mut Vec<VNodeId>` in multiple helpers.

Primary hotspots:

- `src/graph/algorithm/rebalance/resolve.rs`
- `src/graph/algorithm/split/helpers.rs`
- `src/graph/algorithm/evict.rs`

Risk profile:

- Medium risk if queue semantics change.
- Low risk if migration is a pure wrapper + call-site rewrite.

Recommended implementation strategy:

1. Introduce a thin `ViolationQueue` wrapper that delegates to existing push helpers.
2. Migrate call sites incrementally by module.
3. Keep old free functions during migration for compatibility.
4. Remove old plumbing only after all high-value sites are migrated.

## Deferred Pattern

### Pattern 5: Plateau context flattening

Current state:

- Plateau dynamic-tracker hotspots have already been decomposed into helper phases.
- Remaining signatures that pass `gnodes` are explicit and currently stable.

Recommendation:

- Defer larger plateau API redesign unless fresh metrics show clear complexity ROI.

## Success Conditions For Remaining Work

- No behavior changes in violation propagation order.
- Rebalance leaves no residual violations under existing debug audits.
- Complexity does not regress in recently stabilized modules.
    p: VNodeId,
    heaviest: VNodeId,
    violations: &mut Vec<VNodeId>,           // <-- mutating external vector
) { ... }

pub fn push_side_effect_violations<V: Accumulator>(
    vnodes: &Arena<VNode<V>>,
    n: VNodeId,
    violations: &mut Vec<VNodeId>,           // <-- mutating external vector
) { ... }

pub fn push_promoted_violations<V: Accumulator>(
    vnodes: &Arena<VNode<V>>,
    p: VNodeId,
    violations: &mut Vec<VNodeId>,           // <-- mutating external vector
) { ... }
```

**Why It's An Anti-Pattern:**
- Violation queue is owned by VTree/orchestrator
- These functions are "helper utilities" but mutate external state
- Scatters violation logic across many function boundaries

**Refactoring Target:**

```rust
// NEW: Violation queue manager
pub struct ViolationQueue {
    queue: Vec<VNodeId>,
}

impl ViolationQueue {
    pub fn push_contraction_child(&mut self, vnodes: &Arena<VNode<V>>, p: VNodeId, heaviest: VNodeId) { ... }
    pub fn push_side_effects(&mut self, vnodes: &Arena<VNode<V>>, n: VNodeId) { ... }
    pub fn push_promoted(&mut self, vnodes: &Arena<VNode<V>>, p: VNodeId) { ... }
}
```

**Impact:**
- Centralizes violation queue logic
- Eliminates `&mut Vec<VNodeId>` parameter passing
- Makes queue ownership explicit
- Can add queue capacity limits, deduplication, etc.

---

#### 3.2 Basis Consolidation with Vector Mutation
**Files:** `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/consolidate.rs`

```rust
// CURRENT
pub(in super::super) fn collect_subtree_basis_elements(
    &self,
    gnodes: &Arena<GNode<C, V>>,
    gid: GNodeId,
    out: &mut Vec<(GNodeId, u32)>,      // <-- mutating external vector
) { ... }

pub(in super::super) fn place_sorted(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
    elements: &mut [(GNodeId, u32)],    // <-- mutating external slice
) { ... }
```

**Refactoring Target:**

```rust
// NEW: Element collection on tracker
pub struct BasisElementCollector {
    elements: Vec<(GNodeId, u32)>,
}

impl<C, V> DynamicPlateauTracker<C, V> {
    pub fn collect_subtree_elements(
        &self,
        gnodes: &Arena<GNode<C, V>>,
        gid: GNodeId,
    ) -> BasisElementCollector {
        let mut collector = BasisElementCollector::new();
        self.collect_impl(gnodes, gid, &mut collector);
        collector
    }
}
```

---

## Pattern 4: Functions with Similar Parameter Patterns – Missing Container Type

### Anti-Pattern Description
Multiple functions share nearly-identical parameter lists, suggesting a missing container type to group them.

### High-Priority Example

#### 4.1 Plateau Methods All Taking `&Arena<GNode<C, V>>`
**Files:** `src/graph/algorithm/plateau/dynamic_tracker/*.rs`

```rust
// PATTERN: Every method takes gnodes
impl<C, V> DynamicPlateauTracker<C, V> {
    pub(super) fn on_observe_impl(
        &mut self, 
        gnodes: &Arena<GNode<C, V>>,  // <-- Required parameter
        g_id: GNodeId, value: V
    ) { ... }

    pub(super) fn on_bootstrap_split_impl(
        &mut self, 
        gnodes: &Arena<GNode<C, V>>,  // <-- Required parameter
        g_id: GNodeId
    ) { ... }

    pub(super) fn on_catalytic_split_impl(
        &mut self, 
        gnodes: &Arena<GNode<C, V>>,  // <-- Required parameter
        g_id: GNodeId, left_id: GNodeId
    ) { ... }

    pub(super) fn normalize_impl(
        &mut self, 
        gnodes: &Arena<GNode<C, V>>,  // <-- Required parameter
    ) { ... }

    // ... 10+ more methods, all taking gnodes
}
```

**Why It's An Anti-Pattern:**
- Every method signature forced to include gnodes
- Every call site forced to pass gnodes
- Gnodes should be implicit to tracker operations

**Root Cause:**
- `GvGraph` owns both tracker and gnodes
- Methods called via `GvGraph` wrapper methods (see `update_wrappers.rs`)
- Trait (`PlateauTracking`) forces signature with gnodes parameter

**Refactoring Target (Long-term):**

```rust
// NEW: Tracker has access to gnodes context
pub struct PlateauTrackerWithContext<'a, C, V> {
    tracker: &'a mut DynamicPlateauTracker<C, V>,
    gnodes: &'a Arena<GNode<C, V>>,
}

impl<'a, C, V> PlateauTrackerWithContext<'a, C, V> {
    pub fn on_observe(&mut self, g_id: GNodeId, value: V) {
        self.tracker.on_observe_impl(self.gnodes, g_id, value);
    }
    // Wrapper that doesn't repeat gnodes parameter
}
```

**Impact:**
- Eliminates 15+ redundant parameter passes
- Makes gnodes lifecycle implicit
- Reduces method signature noise

---

## Pattern 5: Multiple Node ID Parameters – Context Structure

### Anti-Pattern Description
Functions accepting multiple node IDs that represent related nodes (parent, child, sibling, etc.) should bundle them into a contextual structure.

### Examples

#### 5.1 Eviction with Multiple Node References
**Files:** `src/graph/algorithm/evict.rs`

```rust
// CURRENT (Multiple related node IDs)
pub(crate) fn on_evict(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
    gnode_id: GNodeId,              // <-- Node being evicted
    parent_id: GNodeId,              // <-- Its parent
    parent_state_after: GState,      // <-- Parent's state post-eviction
    parent_lo: C,
    parent_hi: C,
) { ... }
```

**Grouping:**
- `gnode_id` → evicted node
- `parent_id`, `parent_state_after`, `parent_lo`, `parent_hi` → evicted node's parent
- All form a coherent "eviction event"

**Refactoring Target:**

```rust
// NEW: Eviction context
pub struct EvictionContext<C: Coordinate> {
    pub evicted_id: GNodeId,           // Node being removed
    pub parent_id: GNodeId,            // Parent after eviction
    pub parent_state_after: GState,
    pub parent_range: (C, C),          // (lo, hi) grouped
}

impl<C, V, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn evict_node(&mut self, ctx: EvictionContext<C>) { ... }
}
```

---

#### 5.2 Repair with Parent-Child Relationships
**Files:** `src/graph/algorithm/rebalance/resolve.rs`

```rust
// CURRENT
fn escalate_contract_parent<V>(
    vnodes: &mut Arena<VNode<V>>,
    p: VNodeId,                    // Parent
    heaviest: VNodeId,             // Some child
    h_direct: bool,                // Is heaviest a direct child?
    violations: &mut Vec<VNodeId>,
) -> Option<VNodeId> { ... }
```

**Grouping:**
- `p` and `heaviest` form parent-child relationship
- `h_direct` indicates relationship type
- Should be bundled

---

## Pattern 6: Redundant Free Function Helpers

### Low-Priority Anti-Pattern
Functions that exist only to avoid repeating a couple of lines. Usually just call into data structures.

### Examples

#### 6.1 Simple Query Functions
**Files:** `src/graph/algorithm/rebalance/resolve.rs`

```rust
// ANTI-PATTERN: These are trivial wrappers
fn structural_child_count<V: Accumulator>(vnodes: &Arena<VNode<V>>, id: VNodeId) -> usize {
    vnodes.get(id.index()).child_count()  // <-- Just delegates
}

fn any_child_violated<V: Accumulator>(vnodes: &Arena<VNode<V>>, node: VNodeId) -> bool {
    // <-- Just iterates and calls another function
}
```

**Recommendation:**
- Inline into call sites or
- Convert to methods on VNode if conceptually appropriate
- Or keep if they provide semantic clarity (which they mostly do)

---

## Summary: Refactoring Priority Matrix

| Pattern | # Locations | Parameter Reduction | Readability Gain | Priority |
|---------|------------|-------------------|------------------|----------|
| Tell-Don't-Ask (split ops) | 2 | 0 → 1 param | High (encapsulation) | **Critical** |
| Escalation context | 3 | 5-7 → 2 params | Very High | **Critical** |
| Plateau context | 10+ | 3-4 → 1-2 params | High | **High** |
| Violation queue | 15+ | 2 → 1 param | Medium | **High** |
| Coordinate ranges | 20+ | 2 → 1 param | High | **High** |
| Eviction context | 1 | 6 → 2 params | Medium | **Medium** |
| Simple wrappers | 5+ | 0 | Low | **Low** |

---

## Implementation Strategy (by Accidental Complexity Phases)

### Phase 1: API Normalization
- Convert `bootstrap_split`, `catalytic_split` → GvGraph methods
- Eliminates Smell B (Arena helper functions)

### Phase 2: Abstractions
- Create `EscalationContext` struct (rebalance)
- Create `ViolationQueue` type
- Create `BasisElementInfo` struct
- Extracts groupings from parameters

### Phase 3: Reorganization
- Move violation pushing methods to ViolationQueue
- Consolidate plateau logic into tracker without excessive Arena refs
- Extract query context into reusable type

### Phase 4: Stabilization
- Profile reduction in cognitive load (complexity metrics)
- Ensure test coverage on refactored paths

