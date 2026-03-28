use super::{Accumulator, Coordinate};

/// Read-only spatial query interface over a coordinate domain.
pub trait SpatialRead {
    /// Coordinate type used to address cells.
    type Coord: Coordinate;

    /// Accumulator/intensity payload stored in the structure.
    type Accum: Accumulator;

    /// Returns the cell covering `coord`.
    ///
    /// Contract:
    /// - Caller provides a coordinate in the trait's coordinate system.
    /// - Implementations return a valid cell interval/value view for that point.
    fn get(&self, coord: Self::Coord) -> crate::spatial::view::Cell<Self::Coord, Self::Accum>;
}
