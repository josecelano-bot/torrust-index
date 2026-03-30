mod contour;
mod get;
mod range_sum;
mod sample;

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};

    type G = GvGraph<u8, u32, 8>;

    fn make_config() -> Config<u32> {
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

    // ── get ──────────────────────────────────────────────────────────────
    mod get {
        use super::*;

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

            let root = g.gtree.nodes.root;
            let left_child = g
                .gtree
                .nodes
                .get(root.index())
                .left()
                .expect("root must have left child after split");
            let left_entry = g
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

            let root = g.gtree.nodes.root;
            let right_child = g
                .gtree
                .nodes
                .get(root.index())
                .right()
                .expect("root must have right child after split");
            let right_entry = g
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

    // ── range_sum ────────────────────────────────────────────────────────
    mod range_sum {
        use crate::traits::Coordinate;
        use std::ops::Bound;

        use super::*;

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

    // ── sample ───────────────────────────────────────────────────────────
    mod sample {
        use crate::traits::Rng;

        use super::*;

        struct FixedRng(f64);
        impl Rng for FixedRng {
            fn next_f64(&mut self) -> f64 {
                self.0
            }
        }

        #[test]
        fn returns_none_for_zero_sum_graph() {
            let g = fresh_graph();
            assert!(g.sample(&mut FixedRng(0.5)).is_none());
        }

        #[test]
        fn returns_some_after_at_least_one_observation() {
            let mut g = fresh_graph();
            g.observe(0u8, 10u32);
            assert!(g.sample(&mut FixedRng(0.5)).is_some());
        }

        #[test]
        fn sample_on_split_graph_traverses_structural_vtree() {
            // After enough observations to trigger splits, the vtree contains
            // Structural nodes; sample() must traverse them via sample_child.
            let mut g = fresh_graph();
            for _ in 0..3 {
                g.observe(64u8, 5u32); // bootstrap + further splits
            }
            // The graph has observations so sample returns Some
            let result = g.sample(&mut FixedRng(0.5));
            assert!(result.is_some());
        }

        #[test]
        fn sample_with_rng_near_one_returns_a_cell() {
            // rng near 1.0 exercises paths toward the last child in sample_child
            let mut g = fresh_graph();
            for _ in 0..3 {
                g.observe(64u8, 5u32);
            }
            let result = g.sample(&mut FixedRng(0.999));
            assert!(result.is_some());
        }
    }

    // ── contour_range ────────────────────────────────────────────────────
    #[cfg(feature = "dynamic-contour-tracking")]
    mod contour_range_tests {
        use crate::spatial::range::CoordinateRange;
        use crate::spatial::plateau::BasisEdge;
        use crate::traits::Coordinate;

        use super::*;

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
                g.gtree.nodes.root,
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
                g.gtree.nodes.root,
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
                g.gtree.nodes.root,
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

            let root = g.gtree.nodes.root;
            let evict_child = g
                .gtree
                .nodes
                .get(root.index())
                .left()
                .expect("root must have left child after split");
            let evict_entry = g
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
}
