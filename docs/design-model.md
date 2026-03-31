# Design Model

This document describes the current high-level structure of `torrust-mudlark`
after the recent refactors. It focuses on the public surface, the main runtime
objects, module boundaries, and the responsibilities now assigned to each layer.

## 1. Public API surface

```text
torrust-mudlark
|- GvGraph<C, V, const N: u32, T = DefaultTracker<C, V>>
|- DefaultGraph<C, V, const N: u32>
|- Config<V>
|- StructuralConfig
|- GNodeId
|- GState
|- GNodeChildren
|
|- Read / query DTOs
|  |- Node<C, V>
|  |- Span<C, V>
|  |- Cell<C, V>
|  |- Plateau<C, V>
|  |- BasisEdge<C>
|  |- BasisElement<C, V>
|  |- ContourRange<C, V>
|  `- ContourRangeEnergy<V>
|
|- Layered snapshot types
|  |- Pewei<C, V>
|  |- Layer<C, V>
|  |- Transition<C, V>
|  `- Terminal<C, V>
|
`- Traits
   |- Coordinate
   |- Accumulator
   |- Attenuatable
   |- Weighable
   |- Proratable
   |- Inspectable
   |- Observation
   |- ScalableObservation
   |- Rng
   |- SpatialRead
   |- SpatialWrite
   |- TemporalDecay
   |- WeightedSampler
   |- PlateauRead              (feature-gated)
   `- PlateauTracking          (feature-gated)
```

`GvGraph<C, V, N>` is the main user-facing type. `DefaultGraph<C, V, N>` is the
tracker-specialized alias intended for callers that do not want to name the
fourth type parameter.

## 2. Trait model

`GvGraph` is exposed through thin adapters in `src/graph/traits.rs`.

| Trait | Implemented for | Bounds on `V` | Main method |
| --- | --- | --- | --- |
| `SpatialRead` | `GvGraph<C, V, N>` | `Accumulator + Inspectable` | `get(coord)` |
| `SpatialWrite` | `GvGraph<C, V, N>` | `Accumulator + Inspectable` | `observe(coord, delta)` |
| `TemporalDecay` | `GvGraph<C, V, N>` | `Accumulator + Attenuatable + Inspectable` | `decay(root, attenuation, q)` |
| `WeightedSampler` | `GvGraph<C, V, N>` | `Accumulator + Inspectable + Weighable` | `sample(rng)` |
| `PlateauRead` | `GvGraph<..., DynamicPlateauTracker<...>>` | `Accumulator + Inspectable` | `plateaus()` |

The plateau read surface is only available when the
`dynamic-contour-tracking` feature is enabled.

## 3. Runtime objects

### 3.1 Core storage

```text
Arena<T>
  slots: Vec<T>
  occupied: bitset
  free: freelist
  count: u32

GNodeId / VNodeId
  opaque handles backed by NonZeroU32
```

### 3.2 G-tree nodes

```text
GNode<C, V>
  lo, hi: C
  sum, own: V
  left?: GNodeId
  right?: GNodeId
  parent?: GNodeId
  entry?: VNodeId

GState
  Terminal
  SemiInternal
  Internal
```

The `entry` field is a back-reference into the V-tree. A G-node does not embed
the V-node; it only points at the corresponding entry node.

### 3.3 V-tree nodes

```text
VNode<V>
  intensity: V
  parent?: VNodeId
  kind: VKind<V>

VKind<V>
  Entry {
    gnode: GNodeId,
    is_exposed: bool,
    is_evictable: bool,
  }

  Structural {
    children: Children<V>,
    has_evictable: bool,
  }

Children<V>
  Pair { ids: [VNodeId; 2], intensities: [V; 2] }
  Triple { ids: [VNodeId; 3], intensities: [V; 3] }
```

The V-tree is a 2-3 tree. The child container is now an enum (`Pair` or
`Triple`) rather than a packed variable-length structure.

### 3.4 Top-level graph and tracker strategy

```text
StructuralConfig
  depth_create: u32
  depth_evict: u32
  budget?: usize
  alpha_relax: f64
  bounded_eviction: bool

Config<V>
  structural: StructuralConfig
  split_threshold: V

GvGraph<C, V, const N: u32, T = DefaultTracker<C, V>>
  gtree: GTree<C, V, N>
  vtree: VTree<V>
  config: Config<V>
  tracker: T
```

`DefaultTracker<C, V>` resolves to:

- `DynamicPlateauTracker<C, V>` when `dynamic-contour-tracking` is enabled
- `NoopPlateauTracker` otherwise

`GTree` stores structural counts and depth-gate state:

```text
GTree<C, V, N>
  nodes: GNodeTree<C, V>
  live_depth_evict: u32
  live_depth_create: u32
  depth_buffer: u32
  headroom: usize
  soft_limit?: usize
```

`VTree` stores aggregation state and the rebalance queue:

```text
VTree<V>
  nodes: VNodeTree<V>
  violations: Vec<VNodeId>
```

## 4. Layered module architecture

```text
lib.rs
|
|- handle/               opaque IDs
|- arena/                generic slot arena
|- traits/               public trait abstractions
|- nodes/                G-node primitives
|- spatial/              DTOs, plateau types, contour and Pewei output
|- tree/
|  |- gtree/             routing, interval depth helpers, G-sum propagation
|  `- vtree/             intensity propagation, remove-leaf logic, candidate scan
|- graph/
|  |- config             Config and StructuralConfig
|  |- gv_graph           owning graph state and constructors
|  |- traits             trait adapter impls for GvGraph
|  `- algorithm/
|     |- observe         main mutation orchestrator
|     |- split/          bootstrap and catalytic split logic
|     |- rebalance/      violation scan, resolve, contract, context
|     |- promote         standard / skip / legacy promote
|     |- budget          depth gates, bounded eviction loop integration
|     |- evict           tip eviction and teardown
|     |- decay           uniform and selective attenuation
|     |- query/          get, range_sum, contour_range, sample helpers
|     |- extract         Pewei reconstruction / export
|     `- plateau/        PlateauTracking facade + implementations
`- diagnostics/
   |- invariants         public invariant suites
   |- diagnostic         violation auditing / diagnosis
   |- display, dot, dump renderers and exports
   `- plateau_invariants plateau-specific checks
```

### Dependency direction

The dependency flow remains downward:

```text
foundation -> nodes / spatial -> tree -> graph core -> graph algorithms -> diagnostics
```

The main architectural change is that plateau maintenance is now abstracted
behind the `PlateauTracking<C, V>` trait rather than being scattered across many
feature-gated call sites.

## 5. Key execution flows

### 5.1 `observe(coord, delta)`

The current observe pipeline is explicitly structured in 8 phases:

1. Route the observation to a receiving G-node and update `own`
2. Update the backing V-entry, propagate V sums, and enqueue violated ancestors
3. Recompute G sums up to the root
4. Update the plateau tracker (`plateau_after_observe`)
5. Attempt split, then rebalance queued 2-3 violations
6. Adjust `depth_create` / `depth_evict` against the current soft limit
7. Run evictions when over budget, then rebalance again as needed
8. Normalize plateaus and repair the P-I4 one-hop thatch invariant

### 5.2 `get(coord)`

`get` is a fast G-tree query path:

1. Clamp the coordinate into the domain
2. Route through `gtree.nodes.route_to(coord)`
3. Derive the visible interval, including semi-internal uncovered halves
4. Return a `Cell<C, V>`

### 5.3 `sample(rng)`

`sample` descends the V-tree using child intensities as weights, lands on an
entry V-node, then projects the backing G-node interval as a `Cell`.

### 5.4 `decay(root, attenuation, q)`

`decay` now has two distinct strategies:

- `q == 0.0`: uniform decay
- `q > 0.0`: selective per-depth decay

After attenuation, it recomputes G sums, rebuilds V intensities, resolves any
violations, then performs the same post-mutation plateau repair path.

## 6. Rebalancing and promote responsibilities

Rebalancing is now split into smaller, focused modules:

- `rebalance.rs`: queue loop, residual checks, shared helpers
- `rebalance/resolve.rs`: single-violation resolution paths
- `violation_push.rs`: source-specific violation propagation helpers
- `promote.rs`: promote operations

The three promote variants have distinct roles:

- `standard_promote`: pure V-tree restructure of a structural pair
- `skip_promote`: pure V-tree upward move of an entry
- `legacy_promote`: the only promote path that allocates a new G-node

That distinction is important for both algorithm reasoning and documentation.

## 7. Feature-gated architecture

| Feature | Effect |
| --- | --- |
| `dynamic-contour-tracking` | Enables `DynamicPlateauTracker`, `PlateauBasis`, `PlateauRead`, plateau invariants, and incremental plateau maintenance |
| `rand` | Provides the blanket `Rng` implementation bridge for external RNG types |
| `serde` | Adds serialization derives to public types |

When `dynamic-contour-tracking` is disabled, the graph still compiles and runs,
but plateau maintenance becomes a no-op strategy via `NoopPlateauTracker`.
