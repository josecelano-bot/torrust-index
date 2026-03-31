use super::*;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::{Children, VNode};
use crate::tree::vtree::VNodeTree;

fn build_small_vtree() -> (VNodeTree<u32>, VNodeId, VNodeId, VNodeId) {
    let mut vnodes = VNodeTree::from(crate::arena::Arena::new());

    let root = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                10,
                None,
                GNodeId::from_index(0),
                true,
                true,
            ))
            .0,
    );
    let child_a = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                4,
                Some(root),
                GNodeId::from_index(1),
                true,
                true,
            ))
            .0,
    );
    let child_b = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                6,
                Some(root),
                GNodeId::from_index(2),
                true,
                true,
            ))
            .0,
    );

    *vnodes.get_mut(root.index()) =
        VNode::new_structural(10, None, Children::new_2((child_a, 4), (child_b, 6)), true);

    (vnodes, root, child_a, child_b)
}

#[test]
fn diagnose_collapse_sibling_covers_descendant_and_non_descendant_paths() {
    let (vnodes, root, child_a, _child_b) = build_small_vtree();

    diagnose_collapse_sibling(&vnodes, child_a, root);

    let outsider = VNodeId::from_index(99);
    // Non-descendant branch: use a valid node that is not below `root`.
    // Allocate an unparented entry and diagnose against `root`.
    let mut vnodes = vnodes;
    let detached = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(3),
                true,
                true,
            ))
            .0,
    );
    assert_ne!(detached, outsider);
    diagnose_collapse_sibling(&vnodes, detached, root);
}

#[test]
fn log_vtree_ancestry_walks_to_root_without_panicking() {
    let (vnodes, _root, child_a, _child_b) = build_small_vtree();
    log_vtree_ancestry(&vnodes, child_a);
}

#[test]
fn diagnose_missed_violation_hits_root_and_depth_one_early_returns() {
    let mut vnodes = VNodeTree::from(crate::arena::Arena::new());
    let root = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                1u32,
                None,
                GNodeId::from_index(0),
                true,
                true,
            ))
            .0,
    );
    let child = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                1u32,
                Some(root),
                GNodeId::from_index(1),
                true,
                true,
            ))
            .0,
    );

    let ctx = MissedViolationContext {
        evicted_parent: None,
        evicted_parent_child_count: 0,
        collapse_sibling: None,
    };

    diagnose_missed_violation(&vnodes, root, &ctx);
    diagnose_missed_violation(&vnodes, child, &ctx);
}

#[test]
fn diagnose_missed_violation_covers_structural_and_entry_grandparent_paths() {
    // structural grandparent path
    let mut vnodes = VNodeTree::from(crate::arena::Arena::new());
    let gp = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                12u32,
                None,
                GNodeId::from_index(0),
                true,
                true,
            ))
            .0,
    );
    let parent = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                7u32,
                Some(gp),
                GNodeId::from_index(1),
                true,
                true,
            ))
            .0,
    );
    let violated = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                9u32,
                Some(parent),
                GNodeId::from_index(2),
                true,
                true,
            ))
            .0,
    );
    let uncle = VNodeId::from_index(
        vnodes
            .alloc(VNode::new_entry(
                2u32,
                Some(gp),
                GNodeId::from_index(3),
                true,
                true,
            ))
            .0,
    );
    *vnodes.get_mut(gp.index()) = VNode::new_structural(
        9u32,
        None,
        Children::new_2((parent, 7u32), (uncle, 2u32)),
        true,
    );

    let ctx = MissedViolationContext {
        evicted_parent: Some(gp),
        evicted_parent_child_count: 2,
        collapse_sibling: Some(gp),
    };
    diagnose_missed_violation(&vnodes, violated, &ctx);

    // entry grandparent path (uncles = empty vec)
    let mut vnodes2 = VNodeTree::from(crate::arena::Arena::new());
    let gp2 = VNodeId::from_index(
        vnodes2
            .alloc(VNode::new_entry(
                5u32,
                None,
                GNodeId::from_index(10),
                true,
                true,
            ))
            .0,
    );
    let parent2 = VNodeId::from_index(
        vnodes2
            .alloc(VNode::new_entry(
                3u32,
                Some(gp2),
                GNodeId::from_index(11),
                true,
                true,
            ))
            .0,
    );
    let violated2 = VNodeId::from_index(
        vnodes2
            .alloc(VNode::new_entry(
                4u32,
                Some(parent2),
                GNodeId::from_index(12),
                true,
                true,
            ))
            .0,
    );

    let ctx2 = MissedViolationContext {
        evicted_parent: Some(parent2),
        evicted_parent_child_count: 3,
        collapse_sibling: Some(parent2),
    };
    diagnose_missed_violation(&vnodes2, violated2, &ctx2);
}
