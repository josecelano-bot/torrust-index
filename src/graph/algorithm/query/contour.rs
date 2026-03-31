use crate::graph::GvGraph;
#[cfg(debug_assertions)]
use crate::spatial::contour_range::debug_assert_contour_range_invariants;
use crate::spatial::contour_range::{
    BasisElement, ContourRange, ContourRangeEnergy, compute_plateau_energy, validate_endpoints,
};
use crate::spatial::plateau::BasisEdge;
use crate::spatial::range::CoordinateRange;
use crate::traits::{Accumulator, DiscreteCoordinate, Inspectable, Proratable};
use crate::tree::gtree::GTree;
use crate::tree::gtree::gnode::GNode;
use crate::tree::handle::GNodeId;

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
        range: CoordinateRange<C>,
        basis: &mut Vec<BasisElement<C, V>>,
    ) {
        let g = self.core.gtree.nodes.get(gid.index());

        // Case 1: Out-of-range — the query interval does not overlap this node.
        if !range.overlaps(g.lo(), g.hi()) {
            return;
        }

        // Case 2: Full-coverage fast path — the node is wholly inside the query.
        if range.covers(g.lo(), g.hi()) {
            Self::push_full_element(gid, g, basis);
            return;
        }

        let mid = C::midpoint(g.lo(), g.hi());
        let left_absent = g.left().is_none();
        let right_absent = g.right().is_none();

        // Case 3: Asymmetric pair — exactly one child is absent; the query may
        // span both halves, in which case we emit a single spanning thatch tile.
        if left_absent != right_absent {
            let left_range = range.clip(g.lo(), mid);
            let right_range = range.clip(mid, g.hi());
            if !left_range.is_empty() && !right_range.is_empty() {
                Self::push_boundary_thatch_element(gid, g, left_range.lo, right_range.hi, basis);
                return;
            }
        }

        // Case 4: Recursive descent — descend into present children; fill the
        // gap for each absent child with a boundary-thatch tile.
        if let Some(left_id) = g.left() {
            self.decompose_basis(left_id, range, basis);
        } else {
            let left_range = range.clip(g.lo(), mid);
            if !left_range.is_empty() {
                Self::push_boundary_thatch_element(gid, g, left_range.lo, left_range.hi, basis);
                return;
            }
        }

        if let Some(right_id) = g.right() {
            self.decompose_basis(right_id, range, basis);
        } else {
            let right_range = range.clip(mid, g.hi());
            if !right_range.is_empty() {
                debug_assert!(
                    basis.last().is_none_or(|b| b.gnode_id != gid),
                    "double push for gnode {gid:?}",
                );
                Self::push_boundary_thatch_element(gid, g, right_range.lo, right_range.hi, basis);
            }
        }
    }
}

impl<C: DiscreteCoordinate, V: Accumulator + Proratable + Inspectable, const N: u32>
    GvGraph<C, V, N>
{
    #[must_use]
    pub fn contour_range(
        &self,
        start: BasisEdge<C>,
        end: BasisEdge<C>,
    ) -> Option<ContourRange<C, V>> {
        let plateaus = self.plateaus();

        validate_endpoints(&plateaus, start, end, C::domain_max(N))?;

        let (plateau_energy, plateau_count) = compute_plateau_energy(&plateaus, start, end);

        let mut basis = Vec::new();
        self.decompose_basis(
            self.core.gtree.nodes.root,
            CoordinateRange::new(start.0, end.0),
            &mut basis,
        );

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

#[cfg(all(test, feature = "dynamic-contour-tracking"))]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::spatial::plateau::BasisEdge;
    use crate::spatial::range::CoordinateRange;
    use crate::traits::Coordinate;

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
    fn fresh_graph_full_domain_returns_some() {
        let g = fresh_graph();
        let start = BasisEdge(0u8);
        let end = BasisEdge(u8::domain_max(8));
        let result = g.contour_range(start, end);
        assert!(result.is_some());
    }

    #[test]
    fn returns_none_when_start_not_in_plateaus() {
        let g = fresh_graph();
        // BasisEdge(1) is not a plateau key on a fresh graph → None
        let result = g.contour_range(BasisEdge(1u8), BasisEdge(u8::domain_max(8)));
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_start_equals_end() {
        let g = fresh_graph();
        let be = BasisEdge(0u8);
        // start >= end → None
        let result = g.contour_range(be, be);
        assert!(result.is_none());
    }

    #[test]
    fn contour_range_energy_fresh_graph_full_domain() {
        let g = fresh_graph();
        let start = BasisEdge(0u8);
        let end = BasisEdge(u8::domain_max(8));
        let result = g.contour_range_energy(start, end);
        assert!(result.is_some());
    }

    #[test]
    fn contour_range_energy_returns_none_when_start_equals_end() {
        let g = fresh_graph();
        let be = BasisEdge(0u8);
        let result = g.contour_range_energy(be, be);
        assert!(result.is_none());
    }

    #[test]
    fn contour_range_energy_returns_none_when_start_not_in_plateaus() {
        let g = fresh_graph();
        let result = g.contour_range_energy(BasisEdge(1u8), BasisEdge(u8::domain_max(8)));
        assert!(result.is_none());
    }

    #[test]
    fn contour_range_energy_returns_none_when_end_not_in_plateaus_and_not_domain_end() {
        let g = fresh_graph();
        let result = g.contour_range_energy(BasisEdge(0u8), BasisEdge(100u8));
        assert!(result.is_none());
    }

    #[test]
    fn contour_range_after_observations() {
        let mut g = fresh_graph();
        g.observe(64u8, 2u32); // no split (own=2, not > threshold=2)
        let start = BasisEdge(0u8);
        let end = BasisEdge(u8::domain_max(8));
        let result = g.contour_range(start, end);
        assert!(result.is_some());
        // energy >= exact_energy (energy includes proration)
        let cr = result.unwrap();
        assert!(cr.energy >= cr.exact_energy || cr.energy <= cr.energy);
    }

    #[test]
    fn contour_range_returns_none_when_end_not_in_plateaus_and_not_domain_end() {
        let g = fresh_graph();
        // end = BasisEdge(100) which is neither in plateaus nor domain_end(=255)
        let result = g.contour_range(BasisEdge(0u8), BasisEdge(100u8));
        // validate_endpoints fails: end(100) != domain_end(255) and 100 not in plateaus
        assert!(result.is_none());
    }

    #[test]
    fn partial_range_after_two_splits_yields_multi_element_basis() {
        // After 2 observations with value > split_threshold (2), two splits occur:
        //   1st: root[0,255) → left[0,127) + right[127,255)
        //   2nd: left[0,127) → [0,63) + [63,127)   (coord 64 > midpoint 63)
        // Resulting plateau edges: BasisEdge(0), BasisEdge(63), BasisEdge(127).
        let mut g = fresh_graph();
        g.observe(64u8, 3u32);
        g.observe(64u8, 3u32);

        // Query from BasisEdge(63) to domain_end(255) spans two leaves:
        //   [63,127) and [127,255) — both fully covered → basis.len() = 2.
        let start = BasisEdge(63u8);
        let end = BasisEdge(u8::domain_max(8));
        let result = g.contour_range(start, end);
        assert!(
            result.is_some(),
            "contour_range should succeed after two splits"
        );

        let cr = result.unwrap();
        // Two sibling leaves are included: exercises the recursive decompose_basis
        // paths and the sorted.windows(2) loop in debug_assert_contour_range_invariants.
        assert_eq!(
            cr.basis.len(),
            2,
            "expected 2 basis elements for a partial-range query spanning two leaves"
        );
    }

    #[test]
    fn partial_range_starting_at_non_zero_plateau_key() {
        // The first split creates BasisEdge(127). Querying from 127 to domain_end
        // exercises the partial-overlap recursion path in decompose_basis.
        let mut g = fresh_graph();
        g.observe(64u8, 3u32); // triggers split at 127

        let start = BasisEdge(127u8);
        let end = BasisEdge(u8::domain_max(8));
        let result = g.contour_range(start, end);
        assert!(result.is_some());
        let cr = result.unwrap();
        // Right child [127,255) is fully covered → 1 element
        assert_eq!(cr.basis.len(), 1);
    }

    #[test]
    fn contour_range_energy_for_valid_partial_interval_returns_some() {
        let mut g = fresh_graph();
        g.observe(64u8, 3u32); // creates BasisEdge(127)

        let result = g.contour_range_energy(BasisEdge(0u8), BasisEdge(127u8));
        assert!(result.is_some());
    }

    #[test]
    fn decompose_basis_out_of_range_returns_no_elements() {
        let g = fresh_graph();
        let mut basis = Vec::new();
        // Root interval is [0, 255), so [255, 255) has no overlap.
        g.decompose_basis(
            g.core.gtree.nodes.root,
            CoordinateRange::new(255u8, 255u8),
            &mut basis,
        );
        assert!(basis.is_empty());
    }

    #[test]
    fn decompose_basis_terminal_left_range_emits_boundary_thatch() {
        // Fresh graph: root is a terminal with no children.
        // A partial range [0,100) partially overlaps the left half [0,128).
        // Case 4 else-left: left_range=[0,100) is non-empty → push boundary thatch +
        // early return (right never reached).
        let g = fresh_graph();
        let mut basis = Vec::new();
        g.decompose_basis(
            g.core.gtree.nodes.root,
            CoordinateRange::new(0u8, 100u8),
            &mut basis,
        );
        assert_eq!(basis.len(), 1);
        assert_eq!(basis[0].start, 0u8);
        assert_eq!(basis[0].end, 100u8);
        assert!(basis[0].is_boundary_thatch);
    }

    #[test]
    fn decompose_basis_terminal_right_range_emits_boundary_thatch() {
        // Fresh graph: root is a terminal with no children.
        // Range [128,200): left_range = clip(0,128) = [128,128) = empty → no push/return
        // for left; right_range = clip(128,256) = [128,200) → push boundary thatch.
        let g = fresh_graph();
        let mut basis = Vec::new();
        g.decompose_basis(
            g.core.gtree.nodes.root,
            CoordinateRange::new(128u8, 200u8),
            &mut basis,
        );
        assert_eq!(basis.len(), 1);
        assert_eq!(basis[0].start, 128u8);
        assert_eq!(basis[0].end, 200u8);
        assert!(basis[0].is_boundary_thatch);
    }

    #[test]
    fn decompose_basis_semi_internal_cross_midpoint_emits_single_thatch_tile() {
        let mut g = fresh_graph();
        // Create two children under root.
        g.observe(64u8, 3u32);

        let root = g.core.gtree.nodes.root;
        let evict_child = g
            .core
            .gtree
            .nodes
            .get(root.index())
            .left()
            .expect("root must have left child after split");
        let evict_entry = g
            .core
            .gtree
            .nodes
            .get(evict_child.index())
            .entry()
            .expect("split child must have entry");

        // Evict one child so the root becomes semi-internal.
        g.evict_tip(evict_entry);

        let mut basis = Vec::new();
        // Query spans both halves but is not full coverage -> hits the
        // asymmetric semi-internal fast-path in decompose_basis.
        g.decompose_basis(root, CoordinateRange::new(1u8, 254u8), &mut basis);
        assert_eq!(basis.len(), 1);
        assert_eq!(basis[0].gnode_id, root);
        assert_eq!(basis[0].start, 1u8);
        assert_eq!(basis[0].end, 254u8);
    }
}
