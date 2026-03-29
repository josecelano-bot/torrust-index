# Function Signature Inventory (Pattern Hotspots)

Current inventory of the function signatures that matter to the refactoring-pattern initiative.

Last updated: 2026-03-29

Notes:
- This is intentionally hotspot-focused, not a full API dump.
- Signatures below are aligned with the current codebase after Pattern 1-4 completion.

---

## Pattern Status Snapshot

- Pattern 1 (Tell-Don't-Ask split ownership): Completed
- Pattern 2 (Escalation context grouping): Completed
- Pattern 3 (CoordinateRange adoption): Completed
- Pattern 4 (Violation queue ownership): Completed for orchestration paths
- Pattern 5 (Plateau signature flattening): Deferred

---

## Split Ownership Signatures (Pattern 1)

File: `src/graph/algorithm/split.rs`

```rust
impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn attempt_split(&mut self, g_id: GNodeId)
    fn bootstrap_split(&mut self, g_id: GNodeId)
    fn catalytic_split(&mut self, g_id: GNodeId)
}
```

Key call style in use:

```rust
self.bootstrap_split(g_id);
self.catalytic_split(g_id);
```

---

## Rebalance Context Signatures (Pattern 2)

Files:
- `src/graph/algorithm/rebalance/context.rs`
- `src/graph/algorithm/rebalance/resolve.rs`

```rust
pub struct EscalationContext {
    pub parent_id: VNodeId,
    pub grandparent_id: VNodeId,
    pub heaviest_id: VNodeId,
    pub merged_id: Option<VNodeId>,
    pub grandparent_merged_id: Option<VNodeId>,
    pub heaviest_is_direct_child: bool,
}

pub struct VTreeMutContext<'a, V: Accumulator> {
    pub vnodes: &'a mut Arena<VNode<V>>,
    pub violations: &'a mut Vec<VNodeId>,
}
```

---

## Query Range Signatures (Pattern 3)

Files:
- `src/spatial/range.rs`
- `src/graph/algorithm/query/range_sum.rs`
- `src/graph/algorithm/query/contour.rs`

```rust
pub struct CoordinateRange<C: Coordinate> {
    pub lo: C,
    pub hi: C,
}

impl<C: Coordinate> CoordinateRange<C> {
    pub const fn new(lo: C, hi: C) -> Self
    pub fn is_empty(self) -> bool
    pub fn clamp(self, domain_lo: C, domain_hi: C) -> Self
    pub fn overlaps(self, lo: C, hi: C) -> bool
    pub fn covers(self, lo: C, hi: C) -> bool
    pub fn clip(self, lo: C, hi: C) -> Self
}
```

Query helpers now take range objects:

```rust
fn range_sum_inner(&self, gid: GNodeId, range: CoordinateRange<C>) -> V

pub(super) fn decompose_basis(
    &self,
    gid: GNodeId,
    range: CoordinateRange<C>,
    basis: &mut Vec<BasisElement<C, V>>,
)
```

---

## Violation Queue Signatures (Pattern 4)

File: `src/graph/algorithm/violation_push.rs`

```rust
pub struct ViolationQueue<'a> {
    violations: &'a mut Vec<VNodeId>,
}

impl<'a> ViolationQueue<'a> {
    pub fn new(violations: &'a mut Vec<VNodeId>) -> Self
    pub fn push_side_effect<V: Accumulator>(&mut self, vnodes: &Arena<VNode<V>>, node: VNodeId)
    pub fn push_promoted<V: Accumulator>(&mut self, vnodes: &Arena<VNode<V>>, node: VNodeId)
    pub fn push_contraction_child<V: Accumulator>(
        &mut self,
        vnodes: &Arena<VNode<V>>,
        node: VNodeId,
        skip: VNodeId,
    )
    pub fn push_source_10<V: Accumulator>(&mut self, vnodes: &Arena<VNode<V>>, node: VNodeId)
    pub fn push_leaf_removal<V: Accumulator>(&mut self, vnodes: &Arena<VNode<V>>, start: VNodeId)
    pub fn push_collapse<V: Accumulator>(&mut self, vnodes: &Arena<VNode<V>>, sole: VNodeId)
    pub fn push_remaining_sibling<V: Accumulator>(
        &mut self,
        vnodes: &Arena<VNode<V>>,
        parent: VNodeId,
        removed: VNodeId,
    )
    pub fn push_cousin<V: Accumulator>(
        &mut self,
        vnodes: &Arena<VNode<V>>,
        sole: VNodeId,
        grandparent: VNodeId,
    )
}
```

Migrated callers:
- `src/graph/algorithm/rebalance/resolve.rs`
- `src/graph/algorithm/split/helpers.rs`
- `src/graph/algorithm/evict.rs`

Compatibility layer retained:
- Existing `push_*_violations` free functions remain in `violation_push.rs`.
