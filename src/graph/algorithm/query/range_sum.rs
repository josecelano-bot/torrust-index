use crate::graph::GvGraph;
use crate::spatial::range::CoordinateRange;
use crate::traits::{Accumulator, DiscreteCoordinate, Proratable};

impl<C: DiscreteCoordinate, V: Accumulator + Proratable, const N: u32> GvGraph<C, V, N> {
    /// Sum all intensity values in the given coordinate range.
    ///
    /// # Panics
    ///
    /// Panics if any bound of `range` is NaN.
    #[must_use]
    pub fn range_sum<R: std::ops::RangeBounds<C>>(&self, range: R) -> V {
        use std::ops::Bound;

        let lo = match range.start_bound() {
            Bound::Included(&x) => {
                assert!(!x.is_nan(), "range_sum: start bound is NaN");
                x
            }
            Bound::Excluded(&x) => {
                assert!(!x.is_nan(), "range_sum: start bound is NaN");
                x.next_value()
            }
            Bound::Unbounded => C::zero(),
        };
        let hi = match range.end_bound() {
            Bound::Included(&x) => {
                assert!(!x.is_nan(), "range_sum: end bound is NaN");
                x.next_value()
            }
            Bound::Excluded(&x) => {
                assert!(!x.is_nan(), "range_sum: end bound is NaN");
                x
            }
            Bound::Unbounded => C::domain_max(N),
        };

        let domain_lo = C::zero();
        let domain_hi = C::domain_max(N);

        let range = CoordinateRange::new(lo, hi).clamp(domain_lo, domain_hi);

        if range.is_empty() {
            return V::zero();
        }

        self.range_sum_inner(self.gtree.nodes.root, range)
    }

    fn range_sum_inner(&self, gid: crate::handle::GNodeId, range: CoordinateRange<C>) -> V {
        let g = self.gtree.nodes.get(gid.index());
        let node_lo = g.lo();
        let node_hi = g.hi();

        if !range.overlaps(node_lo, node_hi) {
            return V::zero();
        }

        if range.covers(node_lo, node_hi) {
            return g.sum();
        }

        let overlap = range.clip(node_lo, node_hi);

        let node_width = C::width(node_lo, node_hi).to_f64();
        let overlap_width = C::width(overlap.lo, overlap.hi).to_f64();
        let own_prorated = g.own().scale_by(overlap_width / node_width);

        let left_sum = g
            .left()
            .map_or_else(V::zero, |left_id| self.range_sum_inner(left_id, range));
        let right_sum = g
            .right()
            .map_or_else(V::zero, |right_id| self.range_sum_inner(right_id, range));

        V::add(own_prorated, V::add(left_sum, right_sum))
    }
}
