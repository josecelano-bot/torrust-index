# Function Signature Inventory

Complete listing of all module, type, and function signatures in the torrust-index package.

---

## Module: `handle`

### Type: `GNodeId`
- `pub fn from_index(index: usize) -> Self`
- `pub const fn index(self) -> usize`
- `pub fn try_from(value: u32) -> Result<Self, Self::Error>`

### Type: `VNodeId`
- `pub fn from_index(index: usize) -> Self`
- `pub const fn index(self) -> usize`
- `pub fn try_from(value: u32) -> Result<Self, Self::Error>`

---

## Module: `arena`

### Type: `Arena<T>`
- `pub fn new() -> Self`
- `pub fn count(&self) -> usize`
- `pub fn len(&self) -> usize`
- `pub fn is_empty(&self) -> bool`
- `pub fn capacity(&self) -> usize`
- `pub fn free_slots(&self) -> usize`
- `pub fn alloc(&mut self, value: T) -> usize`
- `pub fn dealloc(&mut self, index: usize) -> T`
- `pub fn get(&self, index: usize) -> &T`
- `pub fn get_mut(&mut self, index: usize) -> &mut T`
- `pub fn is_occupied(&self, index: usize) -> bool`
- `pub fn iter_occupied(&self) -> impl Iterator<Item = (usize, &T)>`
- `fn set_occupied(&mut self, index: usize, value: bool)`

---

## Module: `nodes`

### Module: `nodes::gnode`

#### Type: `GNode<C, V>`
- `pub fn left(&self) -> Option<GNodeId>`
- `pub fn right(&self) -> Option<GNodeId>`
- `pub fn parent(&self) -> Option<GNodeId>`
- `pub fn entry(&self) -> Option<VNodeId>`
- `pub fn lo(&self) -> C`
- `pub fn hi(&self) -> C`
- `pub fn sum(&self) -> V`
- `pub fn own(&self) -> V`
- `pub fn new(lo: C, hi: C) -> Self`
- `pub fn new_terminal(lo: C, hi: C) -> Self`
- `pub fn set_left(&mut self, left: Option<GNodeId>)`
- `pub fn set_right(&mut self, right: Option<GNodeId>)`
- `pub fn set_parent(&mut self, parent: Option<GNodeId>)`
- `pub fn set_own(&mut self, own: V)`
- `pub fn set_sum(&mut self, sum: V)`
- `pub fn add_to_own(&mut self, delta: V)`
- `pub fn add_to_sum(&mut self, delta: V)`
- `pub fn detach_child(&mut self, child: GNodeId)`
- `pub fn replace_child(&mut self, old: GNodeId, new: GNodeId)`
- `pub fn clear_child(&mut self, child: GNodeId)`
- `pub const fn assign_entry(&mut self, vid: VNodeId)`
- `pub const fn clear_entry(&mut self)`
- `pub const fn state(&self) -> GState`
- `pub const fn is_terminal(&self) -> bool`
- `pub const fn is_leaf(&self) -> bool`
- `pub const fn has_children(&self) -> bool`
- `pub const fn child_count(&self) -> usize`
- `pub const fn child_ids(&self) -> GNodeChildren`
- `pub const fn has_dependents(&self) -> bool`
- `pub const fn is_semi_internal(&self) -> bool`
- `pub const fn split_into(mut self, left: GNodeId, right: GNodeId) -> Self`
- `pub fn validate(&self) -> Result<(), &'static str>`
- `pub fn uncovered_range(&self) -> Option<(C, C)>`

### Module: `nodes::vnode`

#### Type: `VNode<V>`
- `pub fn intensity(&self) -> V`
- `pub fn parent(&self) -> Option<VNodeId>`
- `pub fn kind(&self) -> &VKind<V>`
- `pub fn new_entry(intensity: V, parent: Option<VNodeId>, gnode: GNodeId, is_exposed: bool, is_evictable: bool) -> Self`
- `pub fn new_structural(intensity: V, parent: Option<VNodeId>, children: Children<V>, has_evictable: bool) -> Self`
- `pub fn is_entry(&self) -> bool`
- `pub fn is_structural(&self) -> bool`
- `pub fn children(&self) -> Option<&Children<V>>`
- `pub fn child_ids(&self) -> Option<&[VNodeId]>`
- `pub const fn child_count(&self) -> usize`
- `pub const fn is_structural_pair(&self) -> bool`
- `pub const fn is_structural_triple(&self) -> bool`
- `pub fn validate(&self) -> Result<(), &'static str>`

#### Type: `Children<V>`
- `pub fn new_2(a: (VNodeId, V), b: (VNodeId, V)) -> Self`
- `pub fn new_3(a: (VNodeId, V), b: (VNodeId, V), c: (VNodeId, V)) -> Self`
- `pub const fn len(&self) -> usize`
- `pub const fn is_empty(&self) -> bool`
- `pub fn find_index(&self, id: VNodeId) -> Option<usize>`
- `pub fn update_intensity(&mut self, index: usize, new: V)`
- `pub fn ids(&self) -> &[VNodeId]`
- `pub const fn get(&self, index: usize) -> (VNodeId, V)` where `V: Copy + PartialOrd`
- `pub fn iter(&self) -> impl Iterator<Item = (VNodeId, V)>`
- `pub fn heaviest_child_index(&self) -> usize`
- `pub fn replace_child(&mut self, old: VNodeId, new: VNodeId, new_intensity: V)`
- `pub fn add_child(&mut self, id: VNodeId, intensity: V)`
- `pub fn remove_child(&mut self, id: VNodeId) -> (VNodeId, V)`

---

## Module: `traits`

### Trait: `Accumulator`
- `fn zero() -> Self`
- `fn add(self, other: Self) -> Self`
- `fn sub(self, other: Self) -> Self`

**Implementations:** `u8, u16, u32, u64, u128, f32, f64`

### Trait: `Coordinate`
- `const BITS: u32`
- `fn zero() -> Self`
- `fn domain_max(n: u32) -> Self`
- `fn midpoint(a: Self, b: Self) -> Self`
- `fn width(start: Self, end: Self) -> Self`
- `fn is_final(start: Self, end: Self, depth: u32, n: u32) -> bool`
- `fn from_u64(v: u64) -> Self`
- `fn to_f64(self) -> f64`
- `fn is_nan(self) -> bool`
- `fn total_cmp(&self, other: &Self) -> std::cmp::Ordering`

**Implementations:** `u8, u16, u32, u64, u128, f32, f64`

### Trait: `DiscreteCoordinate`
- `fn next_value(self) -> Self`

**Implementations:** `u8, u16, u32, u64, u128`

### Trait: `Proratable`
- `fn prorate(self, portion: u64, total: u64) -> Self`
- `fn scale_by(self, ratio: f64) -> Self`

**Implementations:** `u8, u16, u32, u64, u128, f32, f64`

### Trait: `Weighable`
- `fn weight(self) -> f64`

**Implementations:** `u8, u16, u32, u64, u128, f32, f64`

### Trait: `Inspectable`
- `fn to_f64_approx(self) -> f64`
- `fn from_f64(v: f64) -> Self`

**Implementations:** `u8, u16, u32, u64, u128, f32, f64`

### Trait: `Observation<V>`
- `fn accumulate(current: V, delta: Self) -> V`

### Trait: `ScalableObservation<V>`
- `fn scale(current: V, factor: Self) -> V`

### Trait: `SpatialRead`
- Associated Types: `Coord`, `Accum`
- `fn get(&self, coord: Self::Coord) -> Cell<Self::Coord, Self::Accum>`

### Trait: `SpatialWrite`
- `fn observe<O: Observation<Self::Accum>>(&mut self, coord: Self::Coord, delta: O)`

### Trait: `WeightedSampler`
- `fn sample(&self, rng: &mut impl Rng) -> Option<Cell<Self::Coord, Self::Accum>>`

### Trait: `TemporalDecay`
- `fn decay(&mut self, root: GNodeId, attenuation: f64, q: f64)`

### Trait: `PlateauTracking<C, V>`
- `fn on_observe(&mut self, gnodes: &Arena<GNode<C, V>>, g_id: GNodeId, value: V)`
- `fn on_bootstrap_split(&mut self, gnodes: &Arena<GNode<C, V>>, g_id: GNodeId, left_id: GNodeId)`
- `fn on_catalytic_split(&mut self, gnodes: &Arena<GNode<C, V>>, g_id: GNodeId, left_id: GNodeId)`
- `fn on_evict(&mut self, gnodes: &Arena<GNode<C, V>>, gnode_id: GNodeId, parent_id: GNodeId, parent_state_after: GState, parent_lo: C, parent_hi: C)`
- `fn on_legacy_promotes_batched(&mut self, gnodes: &Arena<GNode<C, V>>, new_gnodes: &[GNodeId])`
- `fn normalize(&mut self, gnodes: &Arena<GNode<C, V>>)`
- `fn repair_p_i4(&mut self, gnodes: &Arena<GNode<C, V>>)`
- `fn recompute_sums(&mut self, gnodes: &Arena<GNode<C, V>>, label: &str)`
- `fn set_dirty(&mut self)`
- `fn plateaus(&self) -> Cow<'_, BTreeMap<BasisEdge<C>, Plateau<C, V>>>`
- `fn debug_assert_mirror_consistency(&self, gnodes: &Arena<GNode<C, V>>, fresh: &BTreeMap<BasisEdge<C>, Plateau<C, V>>, label: &str)`

---

## Module: `graph`

### Type: `GvGraph<C, V, N, T>`

#### Query Methods (`graph::algorithm::query::get`)
- `pub fn get(&self, coord: C) -> Cell<C, V>`
- `fn clamp_to_domain(coord: C) -> C`
- `pub(in super::super) fn uncovered_interval(g: &GNode<C, V>) -> (C, C)`
- `fn trimmed_interval(g: &GNode<C, V>, coord: C) -> (C, C)`

#### Range Sum Methods (`graph::algorithm::query::range_sum`)
- `pub fn range_sum<R: RangeBounds<C>>(&self, range: R) -> V`
- `fn range_sum_inner(&self, gid: GNodeId, query_lo: C, query_hi: C) -> V`

#### Contour Methods (`graph::algorithm::query::contour`)
- `fn push_full_element(gid: GNodeId, g: &GNode<C, V>, basis: &mut Vec<BasisElement<C, V>>)`
- `fn push_boundary_thatch_element(gid: GNodeId, g: &GNode<C, V>, start: C, end: C, basis: &mut Vec<BasisElement<C, V>>)`
- `pub(super) fn decompose_basis(&self, gid: GNodeId, query_lo: C, query_hi: C, basis: &mut Vec<BasisElement<C, V>>)`

#### Plateau Read Methods (`graph::algorithm::plateau::read_api`)
- `pub fn plateaus(&self) -> Cow<'_, BTreeMap<BasisEdge<C>, Plateau<C, V>>>`
- `pub fn build_plateaus(&self) -> BTreeMap<BasisEdge<C>, Plateau<C, V>>`

#### Plateau Update Wrappers (`graph::algorithm::plateau::update_wrappers`)
- `pub(crate) fn plateau_after_observe<O: Observation<V>>(&mut self, g_id: GNodeId, delta: O)`
- `pub(crate) fn plateau_after_bootstrap_split(&mut self, g_id: GNodeId, left_id: GNodeId)`
- `pub(crate) fn plateau_after_catalytic_split(&mut self, g_id: GNodeId, left_id: GNodeId)`
- `pub(crate) fn plateau_after_legacy_promotes_batched(&mut self, new_gnodes: &[GNodeId])`
- `pub(crate) fn plateau_after_evict(&mut self, gnode_id: GNodeId, parent_id: GNodeId, parent_state_after: GState, parent_lo: C, parent_hi: C)`
- `pub(crate) fn normalize_plateaus(&mut self)`
- `pub(crate) fn repair_p_i4(&mut self)`
- `pub(crate) fn plateau_recompute_sums(&mut self, label: &str)`
- `pub(crate) fn debug_assert_plateau_mirror_consistency(&self, label: &str)`

#### Plateau Debug API (`graph::algorithm::plateau::debug_api`)
- `pub(crate) const fn plateau_basis(&self) -> &PlateauBasis<C>` (feature: `dynamic-contour-tracking`)
- `pub(crate) fn debug_check_plateau_sums(&self, label: &str)` (feature: `dynamic-contour-tracking`)
- `pub fn debug_plateau_basis(&self) -> Vec<(BasisEdge<C>, Vec<(usize, C, C, &'static str, u32)>)>` (feature: `dynamic-contour-tracking`)

---

## Module: `graph::algorithm::plateau`

### Type: `NoopPlateauTracker`

**Implements `PlateauTracking<C, V>` trait:**
- `fn on_observe(&mut self, _gnodes: &Arena<GNode<C, V>>, _g_id: GNodeId, _value: V)`
- `fn on_bootstrap_split(&mut self, _gnodes: &Arena<GNode<C, V>>, _g_id: GNodeId, _left_id: GNodeId)`
- `fn on_catalytic_split(&mut self, _gnodes: &Arena<GNode<C, V>>, _g_id: GNodeId, _left_id: GNodeId)`
- `fn on_evict(&mut self, _gnodes: &Arena<GNode<C, V>>, _gnode_id: GNodeId, _parent_id: GNodeId, _parent_state_after: GState, _parent_lo: C, _parent_hi: C)`
- `fn on_legacy_promotes_batched(&mut self, _gnodes: &Arena<GNode<C, V>>, _new_gnodes: &[GNodeId])`
- `fn normalize(&mut self, _gnodes: &Arena<GNode<C, V>>)`
- `fn repair_p_i4(&mut self, _gnodes: &Arena<GNode<C, V>>)`
- `fn recompute_sums(&mut self, _gnodes: &Arena<GNode<C, V>>, _label: &str)`
- `fn set_dirty(&mut self)`
- `fn plateaus(&self) -> Cow<'_, BTreeMap<BasisEdge<C>, Plateau<C, V>>>`

### Type: `DynamicPlateauTracker<C, V>` (feature: `dynamic-contour-tracking`)
- `pub(crate) fn with_root(root_key: BasisEdge<C>, root_depth: u32, root_lo: C, root_hi: C, g_root: GNodeId, n_bits: u32) -> Self`

**Implements `PlateauTracking<C, V>` trait (all methods)**

#### Core Helpers - Place (`dynamic_tracker::core_helpers::place`)
- `pub(in super::super) fn place_basis_element(&mut self, gnodes: &Arena<GNode<C, V>>, gnode: GNodeId, depth: u32)`
- `pub(in super::super) fn find_adjacent_left_key(&self, key: BasisEdge<C>, lo: C, depth: u32) -> Option<BasisEdge<C>>`
- `pub(in super::super) fn find_adjacent_right_key(&self, key: BasisEdge<C>, hi: C, depth: u32) -> Option<BasisEdge<C>>`
- `pub(in super::super) fn place_merge_both(&mut self, gnodes: &Arena<GNode<C, V>>, lk: BasisEdge<C>, rk: BasisEdge<C>, gnode: GNodeId)`
- `pub(in super::super) fn place_extend_left(&mut self, gnodes: &Arena<GNode<C, V>>, lk: BasisEdge<C>, gnode: GNodeId)`
- `pub(in super::super) fn place_rekey_right(&mut self, gnodes: &Arena<GNode<C, V>>, key: BasisEdge<C>, rk: BasisEdge<C>, gnode: GNodeId, depth: u32)`
- `pub(in super::super) fn place_new_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, key: BasisEdge<C>, gnode: GNodeId, depth: u32)`

#### Core Helpers - Consolidate (`dynamic_tracker::core_helpers::consolidate`)
- `pub(in super::super) fn collect_subtree_basis_elements(&self, gnodes: &Arena<GNode<C, V>>, gid: GNodeId, out: &mut Vec<(GNodeId, u32)>)`
- `pub(in super::super) fn place_sorted(&mut self, gnodes: &Arena<GNode<C, V>>, elements: &mut [(GNodeId, u32)])`
- `pub(in super::super) fn consolidate_basis_up(&mut self, gnodes: &Arena<GNode<C, V>>, mut gid: GNodeId)`

#### Core Helpers - Fixup (`dynamic_tracker::core_helpers::fixup`)
- `pub(in super::super) fn recompute_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, key: &BasisEdge<C>)`
- `pub(in super::super) fn fixup_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, old_key: BasisEdge<C>)`

#### Split Bootstrap (`dynamic_tracker::split_bootstrap`)
- `pub(super) fn on_bootstrap_split_impl(&mut self, gnodes: &Arena<GNode<C, V>>, g_id: GNodeId)`

#### Split Catalytic (`dynamic_tracker::split_catalytic`)
- `pub(super) fn on_catalytic_split_impl(&mut self, gnodes: &Arena<GNode<C, V>>, g_id: GNodeId, left_id: GNodeId)`

#### Eviction (`dynamic_tracker::evict`)
- `pub(super) fn on_evict_impl(&mut self, gnodes: &Arena<GNode<C, V>>, gnode_id: GNodeId, parent_id: GNodeId, parent_state_after: GState, parent_lo: C, parent_hi: C)`
- `fn evict_ancestor_key(&self, gnodes: &Arena<GNode<C, V>>, parent_id: GNodeId, parent_state_after: GState, displaced: &mut Vec<GNodeId>) -> Option<BasisEdge<C>>`
- `fn evacuate_adjacent_plateaus(&self, gnodes: &Arena<GNode<C, V>>, parent_lo: C, parent_hi: C, parent_depth: u32, displaced_extra: &mut Vec<(GNodeId, u32)>)`

#### Observe (`dynamic_tracker::observe`)
- `pub(super) fn on_observe_impl(&mut self, gnodes: &Arena<GNode<C, V>>, g_id: GNodeId, value: V)`

#### Normalize (`dynamic_tracker::normalize`)
- `pub(super) fn normalize_impl(&mut self, gnodes: &Arena<GNode<C, V>>)`
- `fn collect_normalize_elements(&self, gnodes: &Arena<GNode<C, V>>) -> Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)>`

#### Repair (`dynamic_tracker::repair`)
- `pub(super) fn repair_p_i4_impl(&mut self, gnodes: &Arena<GNode<C, V>>)`
- `fn split_for_p_i4(&mut self, gnodes: &Arena<GNode<C, V>>, key: BasisEdge<C>, child_id: GNodeId)`

#### Legacy Promotes (`dynamic_tracker::legacy_promotes`)
- `pub(super) fn on_legacy_promotes_batched_impl(&mut self, gnodes: &Arena<GNode<C, V>>, new_gnodes: &[GNodeId])`

---

## Module: `graph::algorithm::rebalance`

### Independent Functions
- `pub fn find_violated_nodes<V: Accumulator>(vnodes: &Arena<VNode<V>>) -> Vec<VNodeId>`
- `fn structural_child_count<V: Accumulator>(vnodes: &Arena<VNode<V>>, id: VNodeId) -> usize`
- `fn any_child_violated<V: Accumulator>(vnodes: &Arena<VNode<V>>, node: VNodeId) -> bool`
- `fn escalate_contract_parent<V: Accumulator>(vnodes: &mut Arena<VNode<V>>, p: VNodeId, heaviest: VNodeId, h_direct: bool, violations: &mut Vec<VNodeId>) -> Option<VNodeId>`
- `fn escalate_try_contract_grandparent<V: Accumulator>(vnodes: &mut Arena<VNode<V>>, g: VNodeId, heaviest: VNodeId, merged: VNodeId, h_direct: bool, violations: &mut Vec<VNodeId>) -> (Option<VNodeId>, bool)`
- `fn escalate_skip_promote<V: Accumulator>(vnodes: &mut Arena<VNode<V>>, p: VNodeId, heaviest: VNodeId, g_merged: Option<VNodeId>, violations: &mut Vec<VNodeId>)`

---

## Module: `diagnostics`

### Type: `Gn<'a, C, V>`
- `impl Display`

### Type: `Pl<'a, C, V, N>` (feature: `dynamic-contour-tracking`)
- `impl Display`

### Module: `diagnostics::plateau_invariants` (feature: `dynamic-contour-tracking`)

#### Independent Functions
- `pub fn check_plateau_btreemap_key_consistency<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_plateau_basis_consistency<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_plateau_sum_consistency<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_plateau_depth_consistency<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i1_i_keys_are_contour_steps<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i1_ii_tile_contiguity<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i1_iii_run_contains_tile<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i2_basis_minimality<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i3_basis_disjointness<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i4_thatch_one_hop<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn check_p_i5_thatch_depth<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`

### Module: `diagnostics::invariants`

#### Independent Functions
- `fn check_g_i1_summation<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `fn check_g_i2_variable_fanout<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `fn check_g_i4_entry_consistency<C, V, const N: u32>(graph: &GvGraph<C, V, N>, errors: &mut Vec<String>)`
- `pub fn assert_invariants<C, V, const N: u32>(graph: &GvGraph<C, V, N>)`
- `pub fn check_all_invariants<C, V, const N: u32>(graph: &GvGraph<C, V, N>) -> Vec<String>`

### Module: `diagnostics::dot`

#### Independent Functions
- `const fn state_label(s: GState) -> &'static str`
- `pub fn dump_gtree_dot<C, V, const N: u32>(graph: &GvGraph<C, V, N>, label: &str) -> String`
- `pub fn dump_vtree_dot<C, V>(graph: &GvGraph<C, V, _>) -> String`

### Module: `diagnostics::dump`

#### Independent Functions
- `const fn state_label(s: GState) -> &'static str`
- `fn fmt_optional_gnode(opt: Option<GNodeId>, fallback: &str) -> String`
- `fn ancestor_name(depth: usize) -> String`
- `fn semi_internal_lineage<C, V, const N: u32>(graph: &GvGraph<C, V, N>, gnode: GNodeId) -> String`
- `pub fn dump_gtree<C, V, const N: u32>(graph: &GvGraph<C, V, N>) -> String`
- `pub fn dump_plateaus<C, V, const N: u32>(graph: &GvGraph<C, V, N>) -> String`

### Module: `diagnostics::diagnostic`

#### Independent Functions
- `pub fn audit_violations<V: Accumulator + Inspectable>(vnodes: &Arena<VNode<V>>, violations: &[VNodeId], checkpoint: &str) -> Vec<VNodeId>`
- `pub fn diagnose_missed_violation<V: Accumulator + Inspectable>(vnodes: &Arena<VNode<V>>, violated: VNodeId, context: &MissedViolationContext)`

### Module: `diagnostics::plateau_audit` (feature: `dynamic-contour-tracking`)

#### Independent Functions
- `pub fn audit_plateau_consistency<C, V, const N: u32>(graph: &GvGraph<C, V, N>, checkpoint: &str, context: Option<&PlateauAuditContext>)`

---

## Module: `tree`

### Module: `tree::gtree`

#### Independent Functions (mentioned in files)
- `pub fn route_to(coord: C) -> GNodeId`
- `pub fn uniform_contour_depth(gid: GNodeId) -> Option<u32>`
- `pub const fn depth_of_interval(start: C, end: C) -> u32`

### Module: `tree::vtree`

#### Independent Functions (mentioned in files)
- `pub fn v_depth(vnodes: &Arena<VNode<V>>, id: VNodeId) -> u32`
- `pub fn is_ancestor(vnodes: &Arena<VNode<V>>, ancestor: VNodeId, descendant: VNodeId) -> bool`
- `pub fn propagate_v_sums(vnodes: &mut Arena<VNode<V>>, id: VNodeId) -> V`
- `pub fn sync_intensity_in_parent(vnodes: &mut Arena<VNode<V>>, id: VNodeId, new_intensity: V)`

---

## Module: `spatial`

### Module: `spatial::contour_range`

#### Type: `ContourRange<C, V>`
- Various methods for contour range operations

#### Type: `ContourRangeEnergy`
- Methods for energy computation

#### Independent Functions
- `pub fn compute_plateau_energy<C, V>(...) -> ContourRangeEnergy`
- `pub fn validate_endpoints<C, V>(...) -> Result<(), String>`

### Module: `spatial::plateau`

#### Type: `Plateau<C, V>`
- `pub basis_edge: BasisEdge<C>`
- `pub start: C`
- `pub end: C`
- `pub depth: u32`
- `pub sum: V`

#### Independent Functions
- `pub fn basis_edge_of<C, V>(g: &GNode<C, V>) -> BasisEdge<C>`

### Module: `spatial::view`

#### Type: `Cell<C, V>`
- `pub start: C`
- `pub end: C`
- `pub intensity: V`
- `pub depth: u32`

#### Type: `Span<C>`
- Span representation methods

### Module: `spatial::node`

#### Type: `Node`
- Node representation and methods

### Module: `spatial::pewei`

#### Type: `Pewei`
- Pewei representation and operations

#### Type: `Layer`
- Layer representation

#### Type: `Terminal`
- Terminal marker type

#### Type: `Transition`
- Transition representation

---

## Summary Statistics

- **Total Modules:** 15+
- **Total Trait Definitions:** 13
- **Total Types:** 50+
- **Total Type Methods:** 300+
- **Independent Functions:** 100+

