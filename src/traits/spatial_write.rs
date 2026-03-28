use super::{Observation, SpatialRead};

/// Mutation interface for recording observations into a spatial index.
pub trait SpatialWrite: SpatialRead {
    /// Applies `delta` at `coord`.
    ///
    /// Contract:
    /// - Caller supplies a coordinate and an observation compatible with
    ///   `Self::Accum`.
    /// - Implementations update internal state so subsequent reads include
    ///   the effect of this observation.
    fn observe<O: Observation<Self::Accum>>(&mut self, coord: Self::Coord, delta: O);
}
