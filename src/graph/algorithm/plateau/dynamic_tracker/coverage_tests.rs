use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::GNode;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::PlateauTracking;

fn add_leaf(
    gnodes: &mut Arena<GNode<u8, u32>>,
    lo: u8,
    hi: u8,
    sum: u32,
    parent: Option<GNodeId>,
) -> GNodeId {
    let id = GNodeId::from_index(gnodes.alloc(GNode::new_leaf(lo, hi, 0u32, parent)).0);
    let g = gnodes.get_mut(id.index());
    g.set_own(sum);
    g.set_sum(sum);
    id
}

fn add_node(
    gnodes: &mut Arena<GNode<u8, u32>>,
    lo: u8,
    hi: u8,
    parent: Option<GNodeId>,
) -> GNodeId {
    GNodeId::from_index(gnodes.alloc(GNode::new_leaf(lo, hi, 0u32, parent)).0)
}

fn as_gnodes(nodes: Arena<GNode<u8, u32>>) -> crate::tree::gtree::GNodeTree<u8, u32> {
    crate::tree::gtree::GNodeTree {
        nodes,
        root: GNodeId::from_index(0),
        node_count: 0,
        terminal_count: 0,
    }
}

#[test]
fn collect_normalize_elements_expands_non_uniform_internal_subtree() {
    let mut gnodes = Arena::new();
    let root = add_node(&mut gnodes, 0, 16, None);
    let left = add_leaf(&mut gnodes, 0, 8, 1, Some(root));
    let right = add_node(&mut gnodes, 8, 16, Some(root));
    gnodes.get_mut(root.index()).link_left(left);
    gnodes.get_mut(root.index()).link_right(right);

    let right_left = add_leaf(&mut gnodes, 8, 12, 1, Some(right));
    gnodes.get_mut(right.index()).link_left(right_left);

    let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);
    tracker.plateau_basis.insert(BasisEdge(8), right);
    let gnodes = as_gnodes(gnodes);
    let elems = tracker.collect_normalize_elements(&gnodes);

    assert!(elems.iter().any(|e| e.0 == left));
    assert!(elems.iter().any(|e| e.0 == right));
    assert!(elems.iter().any(|e| e.0 == right_left));
}

#[test]
fn on_legacy_promotes_batched_skips_internal_existing_child() {
    let mut gnodes = Arena::new();
    let parent = add_node(&mut gnodes, 0, 16, None);
    let existing_internal = add_node(&mut gnodes, 0, 8, Some(parent));
    let new_child = add_leaf(&mut gnodes, 8, 16, 1, Some(parent));
    gnodes.get_mut(parent.index()).link_left(existing_internal);
    gnodes.get_mut(parent.index()).link_right(new_child);

    let ex_left = add_leaf(&mut gnodes, 0, 4, 1, Some(existing_internal));
    let ex_right = add_leaf(&mut gnodes, 4, 8, 1, Some(existing_internal));
    gnodes.get_mut(existing_internal.index()).link_left(ex_left);
    gnodes
        .get_mut(existing_internal.index())
        .link_right(ex_right);

    let mut tracker =
        DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, parent, 4);
    let gnodes = as_gnodes(gnodes);
    PlateauTracking::on_legacy_promotes_batched(&mut tracker, &gnodes, &[new_child]);

    assert!(tracker.plateau_basis.plateau_key(new_child).is_some());
}

#[test]
fn recompute_sums_refreshes_plateau_totals_from_basis() {
    let mut gnodes = Arena::new();
    let a = add_leaf(&mut gnodes, 0, 8, 2, None);
    let b = add_leaf(&mut gnodes, 8, 16, 3, None);

    let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, a, 4);
    tracker.plateau_basis.insert(BasisEdge(0), b);
    tracker.plateaus.insert(
        BasisEdge(0),
        Plateau {
            basis_edge: BasisEdge(0),
            start: 0,
            end: 16,
            depth: 1,
            sum: 0,
        },
    );

    let gnodes = as_gnodes(gnodes);
    PlateauTracking::recompute_sums(&mut tracker, &gnodes, "test");

    assert_eq!(tracker.plateaus.get(&BasisEdge(0)).map(|p| p.sum), Some(5));
}

#[test]
#[should_panic(expected = "dynamic-contour-tracking mirror diverged")]
fn debug_assert_mirror_consistency_reports_detailed_divergence() {
    let mut gnodes = Arena::new();
    let root = add_leaf(&mut gnodes, 0, 16, 1, None);

    let tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);

    // Deliberately provide a mismatching fresh map to force divergence path.
    let mut fresh = std::collections::BTreeMap::new();
    fresh.insert(
        BasisEdge(8),
        Plateau {
            basis_edge: BasisEdge(8),
            start: 8,
            end: 16,
            depth: 1,
            sum: 99,
        },
    );

    let gnodes = as_gnodes(gnodes);
    PlateauTracking::debug_assert_mirror_consistency(&tracker, &gnodes, &fresh, "coverage");
}

#[test]
fn set_dirty_plateaus_and_equal_mirror_path_are_exercised() {
    let mut gnodes = Arena::new();
    let root = add_leaf(&mut gnodes, 0, 16, 2, None);

    let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 4);
    PlateauTracking::set_dirty(&mut tracker);
    assert!(tracker.plateaus_dirty);

    let plateaus = PlateauTracking::plateaus(&tracker);
    assert_eq!(plateaus.len(), 1);

    let fresh = tracker.plateaus.clone();
    let gnodes = as_gnodes(gnodes);
    PlateauTracking::debug_assert_mirror_consistency(&tracker, &gnodes, &fresh, "equal");
}

#[test]
fn repair_p_i4_guard_paths_handle_mismatch_unoccupied_and_non_semiinternal() {
    let mut gnodes = Arena::new();
    let root = add_leaf(&mut gnodes, 0, 16, 0, None);
    let removed = add_leaf(&mut gnodes, 16, 32, 0, None);
    let _ = gnodes.dealloc(removed.index());

    let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, root, 5);
    tracker.plateau_basis.insert(BasisEdge(16), removed);

    // 1) key mismatch -> first continue branch
    tracker.pending_p_i4.push((root, BasisEdge(8)));
    // 2) unoccupied gnode -> second continue branch
    tracker.pending_p_i4.push((removed, BasisEdge(16)));
    // 3) occupied but terminal (non-semiinternal) -> third continue branch
    tracker.pending_p_i4.push((root, BasisEdge(0)));

    let gnodes = as_gnodes(gnodes);
    PlateauTracking::repair_p_i4(&mut tracker, &gnodes);
    assert_eq!(tracker.plateau_basis.plateau_key(root), Some(BasisEdge(0)));
}

#[test]
fn legacy_promote_with_parent_in_basis_executes_fixup_branch() {
    let mut gnodes = Arena::new();
    let parent = add_node(&mut gnodes, 0, 16, None);
    let existing = add_leaf(&mut gnodes, 0, 8, 1, Some(parent));
    let promoted = add_leaf(&mut gnodes, 8, 16, 1, Some(parent));
    gnodes.get_mut(parent.index()).link_left(existing);
    gnodes.get_mut(parent.index()).link_right(promoted);

    let mut tracker =
        DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 16, parent, 4);
    assert_eq!(
        tracker.plateau_basis.plateau_key(parent),
        Some(BasisEdge(0))
    );

    let gnodes = as_gnodes(gnodes);
    PlateauTracking::on_legacy_promotes_batched(&mut tracker, &gnodes, &[promoted]);

    assert!(tracker.plateau_basis.basis_count() >= 1);
    assert!(!tracker.plateaus.is_empty());
}

#[test]
fn on_evict_with_ancestor_displacement_executes_phase6_replacement() {
    let mut gnodes = Arena::new();
    let root = add_node(&mut gnodes, 0, 32, None);
    let parent = add_node(&mut gnodes, 0, 16, Some(root));
    let sibling = add_leaf(&mut gnodes, 16, 32, 2, Some(root));
    let survivor = add_leaf(&mut gnodes, 8, 16, 3, Some(parent));
    let evicted = add_leaf(&mut gnodes, 0, 8, 1, Some(parent));

    gnodes.get_mut(root.index()).link_left(parent);
    gnodes.get_mut(root.index()).link_right(sibling);
    gnodes.get_mut(parent.index()).link_right(survivor);

    let mut tracker = DynamicPlateauTracker::<u8, u32>::with_root(BasisEdge(0), 0, 0, 32, root, 5);
    let gnodes = as_gnodes(gnodes);
    PlateauTracking::on_evict(
        &mut tracker,
        &gnodes,
        evicted,
        parent,
        crate::nodes::gnode::GState::SemiInternal,
        0,
        16,
    );

    assert!(tracker.plateau_basis.basis_count() >= 1);
    assert!(!tracker.plateaus.is_empty());
}
