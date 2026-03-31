use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GTree;
use crate::tree::gtree::gnode::GNode;

impl<C: Coordinate, V: Accumulator, const N: u32> GvGraph<C, V, N> {
    /// Return the cell that covers `coord`.
    ///
    /// # Panics
    ///
    /// Panics if `coord` is NaN.
    #[must_use]
    pub fn get(&self, coord: C) -> crate::spatial::view::Cell<C, V> {
        assert!(!coord.is_nan(), "get(): coordinate is NaN");

        let clamped = Self::clamp_to_domain(coord);

        let g_id = self.core.gtree.nodes.route_to(clamped);
        let g = self.core.gtree.nodes.get(g_id.index());

        let (start, end) = Self::trimmed_interval(g, clamped);

        #[cfg(debug_assertions)]
        {
            debug_assert!(
                start <= clamped && clamped < end || (clamped == C::domain_max(N) && start < end),
                "get(): returned cell [{start:?}, {end:?}) does not \
                 contain clamped coord {clamped:?}",
            );
        }

        crate::spatial::view::Cell {
            start,
            end,
            intensity: g.own(),
            depth: GTree::<C, V, N>::depth_of_interval(start, end),
        }
    }

    #[inline]
    fn clamp_to_domain(coord: C) -> C {
        let lo = C::zero();
        let hi = C::domain_max(N);
        if coord < lo {
            lo
        } else if coord >= hi {
            hi
        } else {
            coord
        }
    }

    #[inline]
    pub(in super::super) fn uncovered_interval(g: &GNode<C, V>) -> (C, C) {
        use crate::tree::gtree::gnode::GState;
        match g.state() {
            GState::Terminal | GState::Internal => (g.lo(), g.hi()),
            GState::SemiInternal => {
                let mid = C::midpoint(g.lo(), g.hi());
                if g.left().is_some() {
                    (mid, g.hi())
                } else {
                    (g.lo(), mid)
                }
            }
        }
    }

    #[inline]
    fn trimmed_interval(g: &GNode<C, V>, coord: C) -> (C, C) {
        use crate::tree::gtree::gnode::GState;
        match g.state() {
            GState::Terminal | GState::Internal => (g.lo(), g.hi()),
            GState::SemiInternal => {
                let mid = C::midpoint(g.lo(), g.hi());
                if g.left().is_some() {
                    debug_assert!(
                        coord >= mid,
                        "get(): coord {coord:?} in covered half \
                         [lo={:?}, mid={mid:?}) of semi-internal",
                        g.lo(),
                    );
                    (mid, g.hi())
                } else {
                    debug_assert!(
                        coord < mid,
                        "get(): coord {coord:?} in covered half \
                         [mid={mid:?}, hi={:?}) of semi-internal",
                        g.hi(),
                    );
                    (g.lo(), mid)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};

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
    fn returns_zero_intensity_for_unobserved_coordinate() {
        let g = fresh_graph();
        assert_eq!(g.get(42u8).intensity, 0u32);
    }

    #[test]
    fn returns_observed_intensity_after_single_observation() {
        // delta=2 equals split_threshold so no split is triggered (>2 required)
        let mut g = fresh_graph();
        g.observe(0u8, 2u32);
        assert_eq!(g.get(0u8).intensity, 2u32);
    }

    #[test]
    fn returned_cell_contains_the_queried_coordinate() {
        let g = fresh_graph();
        let coord = 50u8;
        let cell = g.get(coord);
        assert!(coord >= cell.start && coord < cell.end);
    }

    #[test]
    fn coord_at_domain_max_is_clamped_and_returns_cell() {
        // For u8/N=8: domain_max(8) = 255 = u8::MAX
        // coord >= hi (=255) → clamped to 255 inside clamp_to_domain
        let g = fresh_graph();
        use crate::traits::Coordinate;
        let cell = g.get(u8::domain_max(8));
        assert_eq!(cell.intensity, 0u32);
    }

    #[test]
    fn get_on_semi_internal_with_left_absent_returns_left_half_cell() {
        // After a split: root [0,255) → left [0,127) + right [127,255).
        // Evicting the left child makes the root SemiInternal (right only).
        // get(10) where 10 ∈ [0,127) exercises trimmed_interval's
        // SemiInternal path where left is absent → returns (lo, mid).
        let mut g = fresh_graph();
        g.observe(64u8, 3u32); // delta > split_threshold=2 → triggers split

        let root = g.core.gtree.nodes.root;
        let left_child = g
            .core
            .gtree
            .nodes
            .get(root.index())
            .left()
            .expect("root must have left child after split");
        let left_entry = g
            .core
            .gtree
            .nodes
            .get(left_child.index())
            .entry()
            .expect("left child must have a VEntry");
        g.evict_tip(left_entry);

        // coord 10 is in the uncovered [0,127) half of the semi-internal root.
        let cell = g.get(10u8);
        assert_eq!(cell.start, 0u8);
        assert_eq!(cell.end, 127u8); // mid = 0 + (255-0)/2 = 127
    }

    #[test]
    fn get_on_semi_internal_with_right_absent_returns_right_half_cell() {
        // After a split, evict the right child → root is SemiInternal (left only).
        // get(200) where 200 ∈ [127,255) exercises the SemiInternal path
        // where left is present → returns (mid, hi).
        let mut g = fresh_graph();
        g.observe(64u8, 3u32);

        let root = g.core.gtree.nodes.root;
        let right_child = g
            .core
            .gtree
            .nodes
            .get(root.index())
            .right()
            .expect("root must have right child after split");
        let right_entry = g
            .core
            .gtree
            .nodes
            .get(right_child.index())
            .entry()
            .expect("right child must have a VEntry");
        g.evict_tip(right_entry);

        // coord 200 is in the uncovered [127,255) half of the semi-internal root.
        let cell = g.get(200u8);
        assert_eq!(cell.start, 127u8);
        assert_eq!(cell.end, 255u8);
    }
}
