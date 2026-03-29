use crate::graph::GvGraph;
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

        let lo = if lo < domain_lo { domain_lo } else { lo };
        let hi = if hi > domain_hi { domain_hi } else { hi };

        if lo >= hi {
            return V::zero();
        }

        self.range_sum_inner(self.gtree.root, lo, hi)
    }

    fn range_sum_inner(&self, gid: crate::handle::GNodeId, query_lo: C, query_hi: C) -> V {
        let g = self.gtree.nodes.get(gid.index());
        let node_lo = g.lo();
        let node_hi = g.hi();

        if query_lo >= node_hi || query_hi <= node_lo {
            return V::zero();
        }

        if query_lo <= node_lo && query_hi >= node_hi {
            return g.sum();
        }

        let overlap_lo = if query_lo > node_lo { query_lo } else { node_lo };
        let overlap_hi = if query_hi < node_hi { query_hi } else { node_hi };

        let node_width = C::width(node_lo, node_hi).to_f64();
        let overlap_width = C::width(overlap_lo, overlap_hi).to_f64();
        let own_prorated = g.own().scale_by(overlap_width / node_width);

        let left_sum = g.left().map_or_else(V::zero, |left_id| {
            self.range_sum_inner(left_id, query_lo, query_hi)
        });
        let right_sum = g.right().map_or_else(V::zero, |right_id| {
            self.range_sum_inner(right_id, query_lo, query_hi)
        });

        V::add(own_prorated, V::add(left_sum, right_sum))
    }
}
