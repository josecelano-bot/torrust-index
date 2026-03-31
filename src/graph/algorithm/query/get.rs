use crate::graph::GvGraph;
use crate::nodes::gnode::GNode;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GTree;

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
        use crate::nodes::gnode::GState;
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
        use crate::nodes::gnode::GState;
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
