# Refactoring Patterns Analysis

Systematic analysis of accidental complexity patterns in the torrust-index codebase.

---

## Pattern 1: "Tell Don't Ask" – Functions Taking Data They Should Query

### Anti-Pattern Description
Functions accept mutable references to complex data structures and perform operations that should be methods on those structures.

### High-Priority Examples

#### 1.1 Split Operations
**Files:** `src/graph/algorithm/split.rs`

```rust
// CURRENT (Tell-Don't-Ask Anti-Pattern)
fn bootstrap_split<C, V, const N: u32>(graph: &mut GvGraph<C, V, N>, g_id: GNodeId) { ... }
fn catalytic_split<C, V, const N: u32>(graph: &mut GvGraph<C, V, N>, g_id: GNodeId) { ... }
```

**Why It's An Anti-Pattern:**
- Function asks GvGraph for detailed data (`graph.gtree.nodes`, `graph.vtree.nodes`, etc.)
- Should be telling GvGraph: "split yourself at g_id"
- Tight coupling to internal GvGraph structure

**Refactoring Target:**
```rust
// FUTURE (Method on GvGraph)
impl<C, V, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn bootstrap_split(&mut self, g_id: GNodeId) { ... }
    pub(crate) fn catalytic_split(&mut self, g_id: GNodeId) { ... }
}
```

**Impact:**
- Reduces 2 free functions → 2 GvGraph methods
- Encapsulates split logic within GvGraph
- Improves "tell don't ask" principle

---

#### 1.2 Plateau Tracking Operations
**Files:** `src/graph/algorithm/plateau/dynamic_tracker/`

```rust
// CURRENT (Multiple Arena operations scattered)
pub(super) fn on_observe_impl(
    &mut self, 
    gnodes: &Arena<GNode<C, V>>,  // <-- asking for data
    g_id: GNodeId, 
    value: V
) { ... }

pub(super) fn on_bootstrap_split_impl(
    &mut self, 
    gnodes: &Arena<GNode<C, V>>,  // <-- asking for data
    g_id: GNodeId
) { ... }
```

**Why It's An Anti-Pattern:**
- Tracker should not ask for `gnodes` Arena references
- GvGraph owns both the tracker and gnodes
- Forces coupling between tracker and node management

**Refactoring Target:**
```rust
// FUTURE (Tracker methods on GvGraph coordinate transfers)
impl<C, V, N, T> GvGraph<C, V, N, T> {
    pub(crate) fn on_observe(&mut self, g_id: GNodeId, value: V) {
        // Pass filtered references or use callbacks
        self.tracker.on_observe(&self.gtree.nodes, g_id, value);
    }
}
```

---

### Medium-Priority Examples

#### 1.3 V-Tree Helpers Taking Both Mutable and Immutable Trees
**Files:** `src/tree/vtree.rs`

```rust
// CURRENT
pub fn propagate_v_sums<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,  // <-- asking for mutable nodes
    start: VNodeId
) { ... }

pub fn sync_intensity_in_parent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,  // <-- asking for mutable nodes
    id: VNodeId, 
    new_intensity: V
) { ... }
```

**Why It's An Anti-Pattern:**
- These are VTree operations but live as free functions
- Currently accessed via `graph.vtree.propagate_sums(id)`
- Should be methods on a VTree type wrapper

**Refactoring Target:**
```rust
// FUTURE (Methods on VTree)
struct VTree<V> { 
    nodes: Arena<VNode<V>>,
    // ...
}

impl<V> VTree<V> {
    pub fn propagate_sums(&mut self, start: VNodeId) { ... }
    pub fn sync_intensity_in_parent(&mut self, id: VNodeId, new_intensity: V) { ... }
}
```

---

## Pattern 2: Many Parameters – Missing Abstraction

### Anti-Pattern Description
Functions accept multiple parameters that form natural groupings. The grouping itself implies a missing type abstraction.

### High-Priority Examples

#### 2.1 Escalate Operations with Multiple Context Parameters
**Files:** `src/graph/algorithm/rebalance/resolve.rs`

```rust
// CURRENT (5-7 parameters with implicit structure)
fn escalate_contract_parent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,    // Cluster A: Tree context
    p: VNodeId,                        //
    heaviest: VNodeId,                 // Cluster B: Node references
    h_direct: bool,                    // Cluster C: Flags/state
    violations: &mut Vec<VNodeId>,     // Cluster D: Work queue
) -> Option<VNodeId> { ... }

fn escalate_try_contract_grandparent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,      // Cluster A
    g: VNodeId,                         // Cluster B
    heaviest: VNodeId,                  // Cluster B
    merged: VNodeId,                    // Cluster B
    h_direct: bool,                     // Cluster C
    violations: &mut Vec<VNodeId>,      // Cluster D
) -> (Option<VNodeId>, bool) { ... }

fn escalate_skip_promote<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,       // Cluster A
    p: VNodeId,                         // Cluster B
    heaviest: VNodeId,                  // Cluster B
    g_merged: Option<VNodeId>,          // Cluster B
    violations: &mut Vec<VNodeId>,      // Cluster D
) { ... }
```

**Parameter Cluster Analysis:**

| Cluster | Parameters | Abstraction Name | Purpose |
|---------|-----------|------------------|---------|
| A | `vnodes: &mut Arena<VNode<V>>` | `VTreeMut` | Tree structure access |
| B | `p`, `heaviest`, `merged`, `g_merged` | `EscalationContext` | Node references being processed |
| C | `h_direct: bool` | (embedded in context) | Processing flags |
| D | `violations: &mut Vec<VNodeId>` | `ViolationQueue` | Work queue to populate |

**Refactoring Target:**

```rust
// NEW: Contextual grouping
pub struct EscalationContext {
    pub parent_id: VNodeId,
    pub heaviest_id: VNodeId,
    pub merged_id: Option<VNodeId>,
    pub h_direct: bool,
}

pub struct VTreeMutContext<'a, V: Accumulator> {
    pub vnodes: &'a mut Arena<VNode<V>>,
    pub violations: &'a mut Vec<VNodeId>,
}

// REFACTORED
fn escalate_contract_parent<V: Accumulator>(
    tree: &mut VTreeMutContext<V>,
    ctx: &EscalationContext,
) -> Option<VNodeId> { ... }
```

**Impact:**
- Reduces 5-7 parameter functions → 2-parameter functions
- Groups conceptually related node IDs
- Makes work queue explicit
- Improves readability by naming cluster intent

---

#### 2.2 Plateau Placing with Multiple Adjacent-Search Parameters
**Files:** `src/graph/algorithm/plateau/dynamic_tracker/core_helpers/place.rs`

```rust
// CURRENT (Multiple coordinate/depth parameters)
pub(in super::super) fn place_basis_element(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
    gnode: GNodeId,
    depth: u32,
) { ... }

pub(in super::super) fn find_adjacent_left_key(
    &self,
    key: BasisEdge<C>,                 // Current key
    lo: C,                              // Current node's lo boundary
    depth: u32,                         // Current node's depth
) -> Option<BasisEdge<C>> { ... }

pub(in super::super) fn find_adjacent_right_key(
    &self,
    key: BasisEdge<C>,                 // Current key
    hi: C,                              // Current node's hi boundary
    depth: u32,                         // Current node's depth
) -> Option<BasisEdge<C>> { ... }
```

**Parameter Grouping:**

| Grouped As | Parameters | Missing Type Name |
|-----------|-----------|-------------------|
| Element Identity | `gnode, depth, lo, hi` | `BasisElement` |
| Search Context | `key, lo/hi, depth` | `PlacementContext` |

**Refactoring Target:**

```rust
// NEW: Basis element context
pub struct BasisElementInfo<C: Coordinate> {
    pub gnode: GNodeId,
    pub key: BasisEdge<C>,
    pub lo: C,
    pub hi: C,
    pub depth: u32,
}

impl<C, V> DynamicPlateauTracker<C, V> {
    pub(in super::super) fn place_element(&mut self, elem: BasisElementInfo<C>) { ... }
    pub(in super::super) fn find_adjacent_keys(&self, elem: &BasisElementInfo<C>) 
        -> (Option<BasisEdge<C>>, Option<BasisEdge<C>>) { ... }
}
```

**Impact:**
- Groups `(key, lo, hi, depth)` → semantic unit
- Eliminates redundant parameter passing
- Reduces chance of parameter order bugs

---

### Medium-Priority Examples

#### 2.3 Query Operations with Coordinate Range Parameters
**Files:** `src/graph/algorithm/query/`

```rust
// CURRENT (Coordinate ranges passed separately)
fn decompose_basis(
    &self,
    gid: GNodeId,
    query_lo: C,                // Coordinate range
    query_hi: C,                // split across 2 params
    basis: &mut Vec<BasisElement<C, V>>,
) { ... }

fn range_sum_inner(
    &self, 
    gid: GNodeId, 
    query_lo: C,                // Same pattern
    query_hi: C,                // repeated everywhere
) -> V { ... }
```

**Grouping Analysis:**
- `query_lo`, `query_hi` always appear together
- Represent a coordinate range query
- Passed to every recursive call

**Refactoring Target:**

```rust
// NEW: Query range abstraction
pub struct CoordinateRange<C: Coordinate> {
    pub lo: C,
    pub hi: C,
}

// REFACTORED
fn decompose_basis(
    &self,
    gid: GNodeId,
    range: CoordinateRange<C>,
    basis: &mut Vec<BasisElement<C, V>>,
) { ... }
```

---

## Pattern 3: Functions Taking Mutable Collections Should Be Methods

### Anti-Pattern Description
Free functions that mutate collections (particularly `&mut Vec` or `&mut HashMap`) should be methods on the collection owner.

### High-Priority Examples

#### 3.1 Violation Queue Management
**Files:** `src/graph/algorithm/violation_push.rs`

```rust
// CURRENT (Free functions mutating violations vector)
pub fn push_contraction_child_violations<V: Accumulator>(
    vnodes: &Arena<VNode<V>>,
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

