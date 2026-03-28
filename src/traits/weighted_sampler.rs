use super::{Rng, SpatialRead};

/// Random weighted sampling extension over non-zero accumulated mass.
pub trait WeightedSampler: SpatialRead {
    /// Samples one cell according to structure-defined weights.
    ///
    /// Contract:
    /// - Uses `rng` as the randomness source.
    /// - Returns `None` when total sampleable weight is zero.
    /// - Returns `Some(cell)` when a valid weighted sample is available.
    fn sample(
        &self,
        rng: &mut impl Rng,
    ) -> Option<crate::spatial::view::Cell<Self::Coord, Self::Accum>>;
}
