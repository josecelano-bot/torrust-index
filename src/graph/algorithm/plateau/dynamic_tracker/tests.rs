use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::tree::handle::GNodeId;
use crate::tree::gtree::gnode::{GNode, GState};
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::spatial::plateau_basis::PlateauBasis;

fn empty_tracker(n_bits: u32) -> DynamicPlateauTracker<u8, u32> {
    DynamicPlateauTracker {
        plateaus: std::collections::BTreeMap::new(),
        pending_p_i4: Vec::new(),
        plateau_basis: PlateauBasis::new(),
        plateaus_dirty: false,
        n_bits,
    }
}

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

fn add_node(gnodes: &mut Arena<GNode<u8, u32>>, lo: u8, hi: u8, parent: Option<GNodeId>) -> GNodeId {
    GNodeId::from_index(gnodes.alloc(GNode::new_leaf(lo, hi, 0u32, parent)).0)
}

fn as_gnodes(nodes: Arena<GNode<u8, u32>>) -> crate::tree::gtree::GNodeTree<u8, u32> {
    crate::tree::gtree::GNodeTree { nodes, root: GNodeId::from_index(0), node_count: 0, terminal_count: 0 }
}

#[test]
fn with_root_creates_single_root_plateau() {
    let root_key = BasisEdge(0u8);
    let tracker = DynamicPlateauTracker::<u8, u32>::with_root(root_key, 0, 0, 255, GNodeId::from_index(0), 8);

    assert_eq!(tracker.plateaus.len(), 1);
    assert!(tracker.plateaus.contains_key(&root_key));
    assert_eq!(tracker.plateau_basis.plateau_count(), 1);
}

#[test]
fn place_basis_element_rekeys_right_plateau() {
    let mut gnodes = Arena::new();
    let left = add_leaf(&mut gnodes, 0, 8, 2, None);
    let right = add_leaf(&mut gnodes, 8, 16, 3, None);

    let mut tracker = empty_tracker(4);
    tracker.plateau_basis.insert(BasisEdge(8), right);
    tracker.plateaus.insert(
        BasisEdge(8),
        Plateau {
            basis_edge: BasisEdge(8),
            start: 8,
            end: 16,
            depth: 1,
            sum: 3,
        },
    );

    let gnodes = as_gnodes(gnodes);
    tracker.place_basis_element(&gnodes, left, 1);

    assert!(tracker.plateaus.contains_key(&BasisEdge(0)));
    assert!(!tracker.plateaus.contains_key(&BasisEdge(8)));
    assert_eq!(tracker.plateau_basis.plateau_key(left), Some(BasisEdge(0)));
    assert_eq!(tracker.plateau_basis.plateau_key(right), Some(BasisEdge(0)));
}

#[test]
fn place_basis_element_merges_left_and_right_plateaus() {
    let mut gnodes = Arena::new();
    let left = add_leaf(&mut gnodes, 0, 8, 2, None);
    let middle = add_leaf(&mut gnodes, 8, 16, 5, None);
    let right = add_leaf(&mut gnodes, 16, 24, 7, None);

    let mut tracker = empty_tracker(5);
    tracker.plateau_basis.insert(BasisEdge(0), left);
    tracker.plateau_basis.insert(BasisEdge(16), right);
    tracker.plateaus.insert(
        BasisEdge(0),
        Plateau {
            basis_edge: BasisEdge(0),
            start: 0,
            end: 8,
            depth: 2,
            sum: 2,
        },
    );
    tracker.plateaus.insert(
        BasisEdge(16),
        Plateau {
            basis_edge: BasisEdge(16),
            start: 16,
            end: 24,
            depth: 2,
            sum: 7,
        },
    );

    let gnodes = as_gnodes(gnodes);
    tracker.place_basis_element(&gnodes, middle, 2);

    assert!(tracker.plateaus.contains_key(&BasisEdge(0)));
    assert!(!tracker.plateaus.contains_key(&BasisEdge(16)));
    assert_eq!(tracker.plateau_basis.plateau_key(left), Some(BasisEdge(0)));
    assert_eq!(tracker.plateau_basis.plateau_key(middle), Some(BasisEdge(0)));
    assert_eq!(tracker.plateau_basis.plateau_key(right), Some(BasisEdge(0)));
}

#[test]
fn fixup_plateau_rekeys_to_min_basis_edge() {
    let mut gnodes = Arena::new();
    let only = add_leaf(&mut gnodes, 0, 8, 1, None);

    let mut tracker = empty_tracker(4);
    tracker.plateau_basis.insert(BasisEdge(8), only);
    tracker.plateaus.insert(
        BasisEdge(8),
        Plateau {
            basis_edge: BasisEdge(8),
            start: 0,
            end: 8,
            depth: 1,
            sum: 1,
        },
    );

    let gnodes = as_gnodes(gnodes);
    tracker.fixup_plateau(&gnodes, BasisEdge(8));

    assert!(tracker.plateaus.contains_key(&BasisEdge(0)));
    assert!(!tracker.plateaus.contains_key(&BasisEdge(8)));
    assert_eq!(tracker.plateau_basis.plateau_key(only), Some(BasisEdge(0)));
}

#[test]
fn fixup_plateau_evacuates_non_contiguous_remainder() {
    let mut gnodes = Arena::new();
    let a = add_leaf(&mut gnodes, 0, 4, 1, None);
    let b = add_leaf(&mut gnodes, 8, 12, 1, None);

    let mut tracker = empty_tracker(5);
    tracker.plateau_basis.insert(BasisEdge(0), a);
    tracker.plateau_basis.insert(BasisEdge(0), b);
    tracker.plateaus.insert(
        BasisEdge(0),
        Plateau {
            basis_edge: BasisEdge(0),
            start: 0,
            end: 12,
            depth: 3,
            sum: 2,
        },
    );

    let gnodes = as_gnodes(gnodes);
    tracker.fixup_plateau(&gnodes, BasisEdge(0));

    let a_key = tracker.plateau_basis.plateau_key(a);
    let b_key = tracker.plateau_basis.plateau_key(b);
    assert!(a_key.is_some());
    assert!(b_key.is_some());
    assert_ne!(a_key, b_key);
}

#[test]
fn evict_ancestor_key_collects_sibling_and_survivor_for_semi_internal_parent() {
    let mut gnodes = Arena::new();
    let root = add_node(&mut gnodes, 0, 32, None);
    let parent = add_node(&mut gnodes, 0, 16, Some(root));
    let sibling = add_leaf(&mut gnodes, 16, 32, 2, Some(root));
    let child = add_leaf(&mut gnodes, 8, 16, 3, Some(parent));

    gnodes.get_mut(root.index()).link_left(parent);
    gnodes.get_mut(root.index()).link_right(sibling);
    gnodes.get_mut(parent.index()).link_right(child);

    let mut tracker = empty_tracker(5);
    tracker.plateau_basis.insert(BasisEdge(0), root);
    tracker.plateaus.insert(
        BasisEdge(0),
        Plateau {
            basis_edge: BasisEdge(0),
            start: 0,
            end: 32,
            depth: 0,
            sum: 5,
        },
    );

    let mut displaced = Vec::new();
    let gnodes = as_gnodes(gnodes);
    let key = tracker.evict_ancestor_key(&gnodes, parent, GState::SemiInternal, &mut displaced);

    assert_eq!(key, Some(BasisEdge(0)));
    assert_eq!(tracker.plateau_basis.plateau_key(root), None);
    assert!(displaced.contains(&child));
    assert!(displaced.contains(&sibling));
}

#[test]
fn evacuate_adjacent_plateaus_moves_left_and_right_members() {
    let mut gnodes = Arena::new();
    let left = add_leaf(&mut gnodes, 0, 8, 1, None);
    let mid = add_leaf(&mut gnodes, 8, 16, 2, None);
    let right = add_leaf(&mut gnodes, 16, 24, 3, None);

    let mut tracker = empty_tracker(5);
    tracker.plateau_basis.insert(BasisEdge(0), left);
    tracker.plateau_basis.insert(BasisEdge(8), mid);
    tracker.plateau_basis.insert(BasisEdge(16), right);

    for (key, start, end, sum) in [(0u8, 0u8, 8u8, 1u32), (8u8, 8u8, 16u8, 2u32), (16u8, 16u8, 24u8, 3u32)] {
        tracker.plateaus.insert(
            BasisEdge(key),
            Plateau {
                basis_edge: BasisEdge(key),
                start,
                end,
                depth: 2,
                sum,
            },
        );
    }

    let mut displaced = Vec::new();
    let gnodes = as_gnodes(gnodes);
    tracker.evacuate_adjacent_plateaus(&gnodes, 8, 16, 2, &mut displaced);

    assert!(!tracker.plateaus.contains_key(&BasisEdge(0)));
    assert!(tracker.plateaus.contains_key(&BasisEdge(8)));
    assert!(!tracker.plateaus.contains_key(&BasisEdge(16)));
    assert_eq!(displaced.len(), 2);
    assert!(displaced.iter().any(|(gid, _)| *gid == left));
    assert!(displaced.iter().any(|(gid, _)| *gid == right));
}

#[test]
fn repair_p_i4_splits_when_child_stays_in_parent_plateau() {
    let mut gnodes = Arena::new();
    let parent = add_node(&mut gnodes, 0, 16, None);
    let child = add_leaf(&mut gnodes, 8, 16, 4, Some(parent));
    gnodes.get_mut(parent.index()).link_right(child);

    let mut tracker = empty_tracker(4);
    tracker.plateau_basis.insert(BasisEdge(0), parent);
    tracker.plateaus.insert(
        BasisEdge(0),
        Plateau {
            basis_edge: BasisEdge(0),
            start: 0,
            end: 16,
            depth: 0,
            sum: 4,
        },
    );

    let gnodes = as_gnodes(gnodes);
    crate::traits::PlateauTracking::repair_p_i4(&mut tracker, &gnodes);

    assert_eq!(tracker.plateau_basis.plateau_key(child), Some(BasisEdge(8)));
    assert!(tracker.plateaus.contains_key(&BasisEdge(8)));
}

#[test]
fn normalize_rebuilds_from_basis_and_merges_adjacent_equal_depth_tiles() {
    let mut gnodes = Arena::new();
    let a = add_leaf(&mut gnodes, 0, 8, 2, None);
    let b = add_leaf(&mut gnodes, 8, 16, 3, None);

    let mut tracker = empty_tracker(4);
    tracker.plateau_basis.insert(BasisEdge(0), a);
    tracker.plateau_basis.insert(BasisEdge(8), b);
    tracker.plateaus.insert(
        BasisEdge(0),
        Plateau {
            basis_edge: BasisEdge(0),
            start: 0,
            end: 8,
            depth: 1,
            sum: 2,
        },
    );
    tracker.plateaus.insert(
        BasisEdge(8),
        Plateau {
            basis_edge: BasisEdge(8),
            start: 8,
            end: 16,
            depth: 1,
            sum: 3,
        },
    );
    tracker.plateaus_dirty = true;

    let gnodes = as_gnodes(gnodes);
    crate::traits::PlateauTracking::normalize(&mut tracker, &gnodes);

    assert!(!tracker.plateaus_dirty);
    assert_eq!(tracker.plateaus.len(), 1);
    assert!(tracker.plateaus.contains_key(&BasisEdge(0)));
}
