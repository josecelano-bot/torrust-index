use crate::graph::GvGraph;
use crate::handle::GNodeId;
use crate::nodes::gnode::GNode;
#[cfg(debug_assertions)]
use crate::spatial::contour_range::debug_assert_contour_range_invariants;
use crate::spatial::contour_range::{
    BasisElement, ContourRange, ContourRangeEnergy, compute_plateau_energy, validate_endpoints,
};
use crate::spatial::plateau::BasisEdge;
use crate::traits::{Accumulator, DiscreteCoordinate, Inspectable, Proratable};
use crate::tree::gtree::GTree;

impl<C: DiscreteCoordinate, V: Accumulator + Proratable, const N: u32> GvGraph<C, V, N> {
    /// Pushes a full-coverage (non-thatch) element for `gid` onto `basis`.
    fn push_full_element(gid: GNodeId, g: &GNode<C, V>, basis: &mut Vec<BasisElement<C, V>>) {
        basis.push(BasisElement {
            gnode_id: gid,
            start: g.lo(),
            end: g.hi(),
            own: g.own(),
            sum: g.sum(),
            depth: GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi()),
            is_boundary_thatch: false,
        });
    }

    /// Pushes a boundary-thatch element covering `[start, end)` for `gid` onto `basis`.
    fn push_boundary_thatch_element(
        gid: GNodeId,
        g: &GNode<C, V>,
        start: C,
        end: C,
        basis: &mut Vec<BasisElement<C, V>>,
    ) {
        basis.push(BasisElement {
            gnode_id: gid,
            start,
            end,
            own: g.own(),
            sum: g.sum(),
            depth: GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi()),
            is_boundary_thatch: true,
        });
    }

    pub(super) fn decompose_basis(
        &self,
        gid: GNodeId,
        query_lo: C,
        query_hi: C,
        basis: &mut Vec<BasisElement<C, V>>,
    ) {
        let g = self.gtree.nodes.get(gid.index());

        // Case 1: Out-of-range — the query interval does not overlap this node.
        if query_lo >= g.hi() || query_hi <= g.lo() {
            return;
        }

        // Case 2: Full-coverage fast path — the node is wholly inside the query.
        if query_lo <= g.lo() && query_hi >= g.hi() {
            Self::push_full_element(gid, g, basis);
            return;
        }

        let mid = C::midpoint(g.lo(), g.hi());
        let left_absent = g.left().is_none();
        let right_absent = g.right().is_none();

        // Case 3: Asymmetric pair — exactly one child is absent; the query may
        // span both halves, in which case we emit a single spanning thatch tile.
        if left_absent != right_absent {
            let l_lo = if query_lo > g.lo() { query_lo } else { g.lo() };
            let l_hi = if query_hi < mid { query_hi } else { mid };
            let r_lo = if query_lo > mid { query_lo } else { mid };
            let r_hi = if query_hi < g.hi() { query_hi } else { g.hi() };
            if l_lo < l_hi && r_lo < r_hi {
                Self::push_boundary_thatch_element(gid, g, l_lo, r_hi, basis);
                return;
            }
        }

        // Case 4: Recursive descent — descend into present children; fill the
        // gap for each absent child with a boundary-thatch tile.
        if let Some(left_id) = g.left() {
            self.decompose_basis(left_id, query_lo, query_hi, basis);
        } else {
            let tile_lo = if query_lo > g.lo() { query_lo } else { g.lo() };
            let tile_hi = if query_hi < mid { query_hi } else { mid };
            if tile_lo < tile_hi {
                Self::push_boundary_thatch_element(gid, g, tile_lo, tile_hi, basis);
                return;
            }
        }

        if let Some(right_id) = g.right() {
            self.decompose_basis(right_id, query_lo, query_hi, basis);
        } else {
            let tile_lo = if query_lo > mid { query_lo } else { mid };
            let tile_hi = if query_hi < g.hi() { query_hi } else { g.hi() };
            if tile_lo < tile_hi {
                debug_assert!(
                    basis.last().is_none_or(|b| b.gnode_id != gid),
                    "double push for gnode {gid:?}",
                );
                Self::push_boundary_thatch_element(gid, g, tile_lo, tile_hi, basis);
            }
        }
    }
}

impl<C: DiscreteCoordinate, V: Accumulator + Proratable + Inspectable, const N: u32>
    GvGraph<C, V, N>
{
    #[must_use]
    pub fn contour_range(&self, start: BasisEdge<C>, end: BasisEdge<C>) -> Option<ContourRange<C, V>> {
        let plateaus = self.plateaus();

        validate_endpoints(&plateaus, start, end, C::domain_max(N))?;

        let (plateau_energy, plateau_count) = compute_plateau_energy(&plateaus, start, end);

        let mut basis = Vec::new();
        self.decompose_basis(self.gtree.root, start.0, end.0, &mut basis);

        let energy = basis.iter().fold(V::zero(), |acc, b| V::add(acc, b.sum));

        let exact_energy = self.range_sum(start.0..end.0);

        let cross_plateau_energy = V::sub(energy, plateau_energy);

        let result = ContourRange {
            start: start.0,
            end: end.0,
            basis,
            energy,
            exact_energy,
            plateau_energy,
            cross_plateau_energy,
            plateau_count,
        };

        #[cfg(debug_assertions)]
        debug_assert_contour_range_invariants(&result);

        Some(result)
    }

    #[must_use]
    pub fn contour_range_energy(
        &self,
        start: BasisEdge<C>,
        end: BasisEdge<C>,
    ) -> Option<ContourRangeEnergy<V>> {
        let cr = self.contour_range(start, end)?;
        Some(ContourRangeEnergy {
            energy: cr.energy,
            exact_energy: cr.exact_energy,
            plateau_energy: cr.plateau_energy,
            cross_plateau_energy: cr.cross_plateau_energy,
            plateau_count: cr.plateau_count,
        })
    }
}
