use super::SpatialRead;
use crate::spatial::plateau::{BasisEdge, Plateau};

/// Read-only access to contour plateaus (when contour tracking is enabled).
pub trait PlateauRead: SpatialRead {
    /// Iterates plateau runs keyed by their contour-step basis edge.
    ///
    /// Contract:
    /// - Iteration order is key order.
    /// - Each key maps to the plateau active from that step until the next key.
    #[allow(clippy::type_complexity)]
    fn plateaus(
        &self,
    ) -> impl Iterator<Item = (&BasisEdge<Self::Coord>, &Plateau<Self::Coord, Self::Accum>)>;
}
