#![forbid(unsafe_code)]

/// # Public API Contract
///
/// This library provides an adaptive streaming spatial density estimator for 1D coordinate spaces.
/// It maintains a histogram that automatically adjusts resolution based on activity levels,
/// guided by the golden ratio φ, with temporal decay for sliding window tracking.

/// Handle types for identifying nodes in the index.
pub(crate) mod handle;
/// Node implementations for graph and tree structures.
pub(crate) mod nodes;
/// Spatial data structures and algorithms.
pub(crate) mod spatial;

/// Graph-related types and implementations.
pub(crate) mod graph;
/// Trait definitions for the index components.
pub(crate) mod traits;

/// Memory arena for efficient allocations.
pub(crate) mod arena;
/// Diagnostic tools and invariants checking.
pub mod diagnostics;
/// Tree data structures and operations.
pub(crate) mod tree;

#[doc(hidden)]
pub use diagnostics::invariants;

#[doc(hidden)]
pub use graph::GNodeChildren;
pub use graph::{Config, DefaultGraph, GvGraph, StructuralConfig};
pub use handle::GNodeId;
pub use nodes::gnode::GState;
pub use spatial::contour_range::{BasisElement, ContourRange, ContourRangeEnergy};
pub use spatial::node::Node;
pub use spatial::pewei::{Layer, Pewei, Terminal, Transition};
pub use spatial::plateau::{BasisEdge, Plateau};
pub use spatial::view::{Cell, Span};
#[cfg(feature = "dynamic-contour-tracking")]
pub use traits::PlateauRead;
#[cfg(feature = "dynamic-contour-tracking")]
pub use traits::PlateauTracking;
pub use traits::{
    Accumulator, Attenuatable, Coordinate, Inspectable, Observation, Proratable, Rng,
    ScalableObservation, SpatialRead, SpatialWrite, TemporalDecay, Weighable, WeightedSampler,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test_basic_imports() {
        // Test that basic types can be imported and used
        let _id = GNodeId::from_index(0);
        let _state = GState::Terminal;
    }

    #[test]
    fn smoke_test_graph_creation() {
        // Test that a basic graph can be created
        type TestGraph = GvGraph<u32, u64, 16>;
        let config = Config {
            split_threshold: 10,
            structural: StructuralConfig {
                depth_create: 2,
                depth_evict: 5,
                budget: None,
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        };
        let _graph = TestGraph::new(config);
    }
}
