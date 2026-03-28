use super::SpatialRead;

/// Optional time-decay extension for spatial structures.
pub trait TemporalDecay: SpatialRead {
    /// Applies temporal attenuation under the subtree rooted at `root`.
    ///
    /// Contract:
    /// - `attenuation` is the multiplicative factor and `q` is an
    ///   implementation-defined shaping parameter.
    /// - Implementations preserve internal invariants after decay.
    fn decay(&mut self, root: crate::handle::GNodeId, attenuation: f64, q: f64);
}
