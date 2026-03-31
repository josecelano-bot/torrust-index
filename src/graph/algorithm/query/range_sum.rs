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

        self.range_sum_inner(self.core.gtree.nodes.root, range)
    }

    fn range_sum_inner(&self, gid: crate::tree::handle::GNodeId, range: CoordinateRange<C>) -> V {
        let g = self.core.gtree.nodes.get(gid.index());
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

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::traits::Coordinate;
    use std::ops::Bound;

    type G = GvGraph<u8, u32, 8>;

    const fn make_config() -> Config<u32> {
        Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 3,
                depth_evict: 5,
                budget: None,
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        }
    }

    fn fresh_graph() -> G {
        GvGraph::new(make_config())
    }

    #[test]
    fn full_range_equals_total_sum() {
        let mut g = fresh_graph();
        g.observe(0u8, 10u32);
        g.observe(64u8, 20u32);
        let total = g.total_sum();
        let range_total = g.range_sum(0u8..u8::domain_max(8));
        assert_eq!(range_total, total);
    }

    #[test]
    fn empty_range_returns_zero() {
        let mut g = fresh_graph();
        g.observe(0u8, 10u32);
        assert_eq!(g.range_sum(5u8..5u8), 0u32);
    }

    #[test]
    fn sub_range_is_at_most_total_sum() {
        let mut g = fresh_graph();
        g.observe(0u8, 10u32);
        g.observe(200u8, 30u32);
        let sub = g.range_sum(0u8..128u8);
        assert!(sub <= g.total_sum());
    }

    #[test]
    fn unbounded_range_equals_total_sum() {
        let mut g = fresh_graph();
        g.observe(64u8, 15u32);
        assert_eq!(g.range_sum(..), g.total_sum());
    }

    #[test]
    fn range_with_included_end_bound() {
        let mut g = fresh_graph();
        g.observe(64u8, 10u32);
        // x..=y → hi = y.next_value()
        let r = g.range_sum(0u8..=100u8);
        assert!(r <= g.total_sum());
    }

    #[test]
    fn range_with_excluded_start_bound() {
        let mut g = fresh_graph();
        g.observe(64u8, 10u32);
        // Excluded start → lo = x.next_value()
        let r = g.range_sum((Bound::Excluded(0u8), Bound::Unbounded));
        assert!(r <= g.total_sum());
    }

    #[test]
    fn range_with_excluded_end_bound() {
        let mut g = fresh_graph();
        g.observe(64u8, 10u32);
        let r = g.range_sum((Bound::Included(0u8), Bound::Excluded(100u8)));
        assert!(r <= g.total_sum());
    }

    #[test]
    fn unbounded_start_with_finite_end_is_valid() {
        let mut g = fresh_graph();
        g.observe(64u8, 10u32);
        let r = g.range_sum(..100u8);
        assert!(r <= g.total_sum());
    }

    #[test]
    fn range_with_uncovered_interval_returns_zero() {
        let g = fresh_graph();
        // lo >= hi after bounds processing → returns 0
        let r = g.range_sum(100u8..50u8);
        assert_eq!(r, 0u32);
    }

    #[test]
    fn split_tree_range_misses_right_child() {
        let mut g = fresh_graph();
        // delta=5 > split_threshold=2 → forces the root [0,256) to split into
        // a left child [0,128) and a right child [128,256).
        g.observe(10u8, 5u32);
        // Range [0,100) partially overlaps root → range_sum_inner recurses into
        // both children.  Left child [0,128) overlaps the range; right child
        // [128,256) does NOT overlap → exercises the early "return V::zero()"
        // path inside a non-root node.
        let r = g.range_sum(0u8..100u8);
        assert!(r > 0u32);
        assert!(r <= g.total_sum());
    }

    #[test]
    fn split_tree_range_covers_left_child_entirely() {
        let mut g = fresh_graph();
        g.observe(10u8, 5u32);
        // Range [0,200) covers the left child [0,128) entirely →
        // exercises the "range.covers(node_lo, node_hi)" fast path in a child.
        let r = g.range_sum(0u8..200u8);
        assert!(r > 0u32);
        assert!(r <= g.total_sum());
    }
}
