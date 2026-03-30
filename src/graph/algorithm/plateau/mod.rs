//! Plateau management — thin wrappers and public API.

mod debug_api;
mod read_api;
mod update_wrappers;

pub mod dynamic_tracker;
pub mod noop_tracker;

#[cfg(feature = "dynamic-contour-tracking")]
#[allow(unused_imports)]
pub use dynamic_tracker::DynamicPlateauTracker;
#[allow(unused_imports)]
pub use noop_tracker::NoopPlateauTracker;

#[cfg(test)]
mod tests {
    use crate::arena::Arena;
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::handle::GNodeId;
    use crate::nodes::gnode::GNode;
    use crate::spatial::plateau::BasisEdge;
    use crate::traits::PlateauTracking;

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

    fn fresh() -> G {
        GvGraph::new(make_config())
    }

    fn add_leaf(
        gnodes: &mut Arena<GNode<u8, u32>>,
        lo: u8,
        hi: u8,
        sum: u32,
        parent: Option<GNodeId>,
    ) -> GNodeId {
        let id = GNodeId::from_index(gnodes.alloc(GNode::new_leaf(lo, hi, 0u32, parent)));
        let g = gnodes.get_mut(id.index());
        g.set_own(sum);
        g.set_sum(sum);
        id
    }

    fn add_node(gnodes: &mut Arena<GNode<u8, u32>>, lo: u8, hi: u8, parent: Option<GNodeId>) -> GNodeId {
        GNodeId::from_index(gnodes.alloc(GNode::new_leaf(lo, hi, 0u32, parent)))
    }

    fn as_gnodes(nodes: Arena<GNode<u8, u32>>) -> crate::tree::gtree::GNodeTree<u8, u32> {
        crate::tree::gtree::GNodeTree { nodes, root: GNodeId::from_index(0), node_count: 0, terminal_count: 0 }
    }

    // ── build_plateaus ────────────────────────────────────────────────
    mod build_plateaus_fn {
        use super::*;

        #[test]
        fn fresh_graph_has_exactly_one_plateau() {
            let g = fresh();
            let p = g.build_plateaus();
            assert_eq!(p.len(), 1);
        }

        #[test]
        fn fresh_plateau_sum_is_zero() {
            let g = fresh();
            let p = g.build_plateaus();
            let total: u32 = p.values().map(|pl| pl.sum).sum();
            assert_eq!(total, 0u32);
        }

        #[test]
        fn plateau_sum_equals_total_sum_after_observations() {
            let mut g = fresh();
            g.observe(32u8, 3u32);
            g.observe(192u8, 5u32);
            let expected = g.total_sum();
            let p = g.build_plateaus();
            let total: u32 = p.values().map(|pl| pl.sum).sum();
            assert_eq!(total, expected);
        }

        #[test]
        fn plateau_count_grows_after_splits() {
            let mut g = fresh();
            let before = g.build_plateaus().len();
            g.observe(64u8, 3u32); // bootstrap split
            let after = g.build_plateaus().len();
            assert!(after >= before, "split must not reduce plateau count");
        }
    }

    // ── plateaus (public accessor) ────────────────────────────────────
    mod plateaus_fn {
        use super::*;

        #[test]
        fn plateaus_is_consistent_with_build_plateaus() {
            let mut g = fresh();
            g.observe(64u8, 3u32);
            // Without dynamic-contour-tracking, plateaus() == build_plateaus().
            assert_eq!(g.plateaus().len(), g.build_plateaus().len());
        }
    }

    // ── select_plateaus ───────────────────────────────────────────────
    mod select_plateaus_fn {
        use super::*;

        #[test]
        fn returns_none_when_lo_equals_hi() {
            let g = fresh();
            assert!(g.select_plateaus(64u8, 64u8).is_none());
        }

        #[test]
        fn returns_none_when_lo_greater_than_hi() {
            let g = fresh();
            assert!(g.select_plateaus(200u8, 10u8).is_none());
        }

        #[test]
        fn returns_some_for_valid_range_in_observed_graph() {
            let mut g = fresh();
            g.observe(32u8, 3u32); // bootstrap split creates two plateaus
            // Select a sub-range that lies within one side
            let result = g.select_plateaus(0u8, 128u8);
            assert!(result.is_some(), "must find a covering plateau pair");
        }

        #[test]
        fn start_key_is_at_most_end_key() {
            let mut g = fresh();
            g.observe(32u8, 3u32);
            if let Some((start, end)) = g.select_plateaus(0u8, 128u8) {
                assert!(start <= end);
            }
        }

        #[test]
        fn start_key_encodes_lo_of_covering_plateau() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // bootstrap: plateaus at 0 and 128
            // Asking for range [0, 64) must be covered by the first plateau
            let result = g.select_plateaus(0u8, 64u8);
            assert!(result.is_some());
            let (start, _end) = result.unwrap();
            assert_eq!(start, BasisEdge(0u8));
        }
    }

    #[cfg(feature = "dynamic-contour-tracking")]
    mod dynamic_tracker_behavior_fn {
        use super::*;
        use crate::graph::algorithm::plateau::DynamicPlateauTracker;
        use crate::nodes::gnode::GState;
        use crate::spatial::plateau::Plateau;

        #[test]
        fn on_observe_updates_matching_plateau_sums_along_path() {
            let mut gnodes = Arena::new();
            let root = add_node(&mut gnodes, 0, 16, None);
            let child = add_leaf(&mut gnodes, 8, 16, 7, Some(root));
            gnodes.get_mut(root.index()).link_right(child);
            gnodes.get_mut(root.index()).set_sum(7);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);
            tracker.plateau_basis.insert(BasisEdge(8), child);
            tracker.plateaus.insert(
                BasisEdge(8),
                Plateau {
                    basis_edge: BasisEdge(8),
                    start: 8,
                    end: 16,
                    depth: 1,
                    sum: 0,
                },
            );

            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_observe(&mut tracker, &gnodes, child, 7);

            assert_eq!(tracker.plateaus.get(&BasisEdge(0)).map(|p| p.sum), Some(7));
            assert_eq!(tracker.plateaus.get(&BasisEdge(8)).map(|p| p.sum), Some(7));
        }

        #[test]
        fn on_bootstrap_split_handles_unequal_child_depths() {
            let mut gnodes = Arena::new();
            let root = add_node(&mut gnodes, 0, 16, None);
            let left = add_leaf(&mut gnodes, 0, 4, 0, Some(root));
            let right = add_leaf(&mut gnodes, 4, 16, 0, Some(root));
            gnodes.get_mut(root.index()).link_left(left);
            gnodes.get_mut(root.index()).link_right(right);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_bootstrap_split(&mut tracker, &gnodes, root, left);

            assert_eq!(tracker.plateau_basis.plateau_key(root), None);
            assert!(tracker.plateau_basis.plateau_key(left).is_some());
            assert!(tracker.plateau_basis.plateau_key(right).is_some());
        }

        #[test]
        fn on_catalytic_split_uses_covering_ancestor_when_target_not_in_basis() {
            let mut gnodes = Arena::new();
            let root = add_node(&mut gnodes, 0, 32, None);
            let g_id = add_node(&mut gnodes, 0, 16, Some(root));
            let sibling = add_leaf(&mut gnodes, 16, 32, 0, Some(root));
            gnodes.get_mut(root.index()).link_left(g_id);
            gnodes.get_mut(root.index()).link_right(sibling);

            let left = add_leaf(&mut gnodes, 0, 8, 0, Some(g_id));
            let right = add_leaf(&mut gnodes, 8, 16, 0, Some(g_id));
            gnodes.get_mut(g_id.index()).link_left(left);
            gnodes.get_mut(g_id.index()).link_right(right);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 32, root, 5);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_catalytic_split(&mut tracker, &gnodes, g_id, left);

            assert_eq!(tracker.plateau_basis.plateau_key(root), None);
            assert!(tracker.plateau_basis.plateau_key(left).is_some() || tracker.plateau_basis.plateau_key(g_id).is_some());
        }

        #[test]
        fn on_evict_replaces_parent_and_survivor_when_parent_is_in_basis() {
            let mut gnodes = Arena::new();
            let parent = add_node(&mut gnodes, 0, 16, None);
            let survivor = add_leaf(&mut gnodes, 8, 16, 2, Some(parent));
            let evicted = add_leaf(&mut gnodes, 0, 8, 1, Some(parent));
            gnodes.get_mut(parent.index()).link_right(survivor);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, parent, 4);
            tracker.plateau_basis.insert(BasisEdge(0), evicted);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_evict(
                &mut tracker,
                &gnodes,
                evicted,
                parent,
                GState::SemiInternal,
                0,
                16,
            );

            assert!(tracker.plateau_basis.plateau_key(parent).is_some());
            assert!(tracker.plateau_basis.plateau_key(survivor).is_some());
        }

        #[test]
        fn on_legacy_promotes_batched_places_existing_and_new_children() {
            let mut gnodes = Arena::new();
            let parent = add_node(&mut gnodes, 0, 16, None);
            let existing = add_leaf(&mut gnodes, 0, 8, 1, Some(parent));
            let new_child = add_leaf(&mut gnodes, 8, 16, 1, Some(parent));
            gnodes.get_mut(parent.index()).link_left(existing);
            gnodes.get_mut(parent.index()).link_right(new_child);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, parent, 4);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_legacy_promotes_batched(&mut tracker, &gnodes, &[new_child]);

            assert!(tracker.plateaus_dirty);
            assert!(tracker.plateau_basis.basis_count() >= 1);
            assert!(tracker.plateau_basis.plateau_key(parent).is_some() || tracker.plateau_basis.plateau_key(new_child).is_some());
            assert!(!tracker.plateaus.is_empty());
        }

        #[test]
        fn normalize_and_repair_cover_noop_and_skip_paths() {
            let mut gnodes = Arena::new();
            let lone = add_leaf(&mut gnodes, 0, 16, 3, None);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, lone, 4);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::normalize(&mut tracker, &gnodes);

            tracker.pending_p_i4.push((lone, BasisEdge(8)));
            PlateauTracking::repair_p_i4(&mut tracker, &gnodes);

            assert_eq!(tracker.plateau_basis.plateau_key(lone), Some(BasisEdge(0)));
        }

        #[test]
        fn on_bootstrap_split_equal_depth_keeps_parent_as_basis_element() {
            let mut gnodes = Arena::new();
            let root = add_node(&mut gnodes, 0, 16, None);
            let left = add_leaf(&mut gnodes, 0, 8, 0, Some(root));
            let right = add_leaf(&mut gnodes, 8, 16, 0, Some(root));
            gnodes.get_mut(root.index()).link_left(left);
            gnodes.get_mut(root.index()).link_right(right);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_bootstrap_split(&mut tracker, &gnodes, root, left);

            assert_eq!(tracker.plateau_basis.plateau_key(root), Some(BasisEdge(0)));
        }

        #[test]
        fn on_catalytic_split_equal_depth_reinserts_parent() {
            let mut gnodes = Arena::new();
            let root = add_node(&mut gnodes, 0, 16, None);
            let left = add_leaf(&mut gnodes, 0, 8, 0, Some(root));
            let right = add_leaf(&mut gnodes, 8, 16, 0, Some(root));
            gnodes.get_mut(root.index()).link_left(left);
            gnodes.get_mut(root.index()).link_right(right);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);
            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_catalytic_split(&mut tracker, &gnodes, root, left);

            assert!(tracker.plateau_basis.plateau_key(root).is_some());
        }

        #[test]
        fn on_legacy_promotes_batched_empty_input_is_noop() {
            let mut gnodes = Arena::new();
            let root = add_leaf(&mut gnodes, 0, 16, 0, None);
            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);

            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_legacy_promotes_batched(&mut tracker, &gnodes, &[]);

            assert!(!tracker.plateaus_dirty);
            assert_eq!(tracker.plateau_basis.plateau_key(root), Some(BasisEdge(0)));
        }

        #[test]
        #[should_panic(expected = "evict_tip: parent cannot remain Internal after eviction")]
        fn on_evict_panics_when_parent_state_after_is_internal() {
            let mut gnodes = Arena::new();
            let parent = add_node(&mut gnodes, 0, 16, None);
            let left = add_leaf(&mut gnodes, 0, 8, 1, Some(parent));
            let right = add_leaf(&mut gnodes, 8, 16, 1, Some(parent));
            gnodes.get_mut(parent.index()).link_left(left);
            gnodes.get_mut(parent.index()).link_right(right);

            let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, parent, 4);
            tracker.plateau_basis.insert(BasisEdge(0), left);

            let gnodes = as_gnodes(gnodes);
            PlateauTracking::on_evict(&mut tracker, &gnodes, left, parent, GState::Internal, 0, 16);
        }
    }
}
