//! V-tree — intensity-aggregation tree overlaid on the G-tree.
//!
//! The V-tree is a binary tree whose leaves (`VKind::Entry`) correspond
//! one-to-one with live G-node entry points.  Internal nodes
//! (`VKind::Structural`) aggregate the intensities of their children so that
//! any ancestor query can be answered in O(depth) time.
//!
//! ## Node kinds
//!
//! - **`VKind::Entry`**: a leaf that stores the intensity contributed by one
//!   G-node.  It also carries the `GNodeId` it belongs to, enabling the
//!   G-tree to look up its V-node in O(1).
//! - **`VKind::Structural`**: an internal node whose `intensity` is always
//!   the sum of all descendant entry intensities.  Its `children` array holds
//!   up to 3 entries (2 in the balanced case; 3 is transient and triggers a
//!   rebalance violation).
//!
//! ## Update patterns
//!
//! Three contexts require different amounts of work:
//!
//! 1. **Point update + ancestor propagate** (`observe`): after a single
//!    G-node's own value changes, call [`VTree::sync_intensity_in_parent`] to
//!    update the entry's cached slot in its parent, then
//!    [`VTree::propagate_v_sums`] to walk
//!    up to the root recomputing structural intensities.  O(depth) work.
//!
//! 2. **Full post-order recompute** (`decay`): after a bulk operation that
//!    changes many G-nodes at once, call [`VTree::recompute_all_v_intensities`]
//!    which
//!    visits every V-node in post-order.  O(n) work, but correct regardless of
//!    which entries changed.
//!
//! 3. **Structural changes** (`split`, `evict`): inserts / removes reparent
//!    nodes.  Depth is always computed on demand by walking the parent chain
//!    via [`v_depth`] — no cache to invalidate.
//!
//! ## Rebalance violations
//!
//! A V-node is *violated* when its intensity distribution across children
//! breaches the configured balance threshold.  See `rebalance.rs` for the
//! `is_violated` predicate and the `resolve` function that repairs violations.

use crate::handle::{GNodeId, VNodeId};
use crate::tree::gtree::GTree;
use crate::traits::{Accumulator, Coordinate};

pub mod vnode;
pub mod vnode_tree;
pub use vnode_tree::VNodeTree;
use self::vnode::VKind;

// ── VTree ────────────────────────────────────────────────────────────────────

/// The V-tree: an intensity-aggregation binary tree overlaid on the G-tree.
/// Owns the node arena and violations queue. Root identity is managed by [`VNodeTree`].
#[derive(Debug, Clone)]
pub struct VTree<V: Accumulator> {
    /// Backing store for all V-nodes (includes root).
    pub(crate) nodes: VNodeTree<V>,
    /// V-nodes whose intensity distribution violates the balance threshold.
    pub(crate) violations: Vec<VNodeId>,
}

#[derive(Debug, Clone, Copy)]
enum RemoveLeafCase {
    Root,
    Shrink {
        parent: VNodeId,
    },
    Collapse {
        parent: VNodeId,
        sibling: VNodeId,
        grandparent: Option<VNodeId>,
    },
}

// ── VTree methods ─────────────────────────────────────────────────────────────

impl<V: Accumulator> VTree<V> {
    // ── Violations queue ──────────────────────────────────────────────────

    /// Enqueues `id` as a pending violation.
    pub(crate) fn push_violation(&mut self, id: VNodeId) {
        self.violations.push(id);
    }

    // ── Structural modifications ──────────────────────────────────────────

    /// Removes leaf `v_id` from the tree and updates `self.root` in place.
    pub(crate) fn remove_leaf<C: Coordinate, const N: u32>(
        &mut self,
        gtree: &mut GTree<C, V, N>,
        v_id: VNodeId,
    ) {
        let span = tracing::debug_span!(
            "vtree_remove_leaf",
            v_id = v_id.index(),
            case = tracing::field::Empty,
        )
        .entered();

        if let VKind::Entry { gnode, .. } = self.nodes.get(v_id.index()).kind() {
            gtree.nodes.clear_entry(*gnode);
        }

        match self.classify_remove_leaf_case(v_id) {
            RemoveLeafCase::Root => {
                span.record("case", "root");
                self.remove_root_leaf(v_id);
            }
            RemoveLeafCase::Shrink { parent } => {
                span.record("case", "shrink");
                self.remove_from_three_child_parent(parent, v_id);
            }
            RemoveLeafCase::Collapse {
                parent,
                sibling,
                grandparent,
            } => {
                span.record("case", "collapse");
                self.collapse_two_child_parent(parent, sibling, grandparent, v_id);
            }
        }
    }

    fn classify_remove_leaf_case(&self, v_id: VNodeId) -> RemoveLeafCase {
        let Some(p_id) = self.nodes.get(v_id.index()).parent() else {
            return RemoveLeafCase::Root;
        };

        let p = self.nodes.get(p_id.index());
        let p_child_count = match &p.kind() {
            VKind::Structural { children, .. } => children.len(),
            VKind::Entry { .. } => unreachable!("parent of entry should be structural"),
        };

        if p_child_count == 3 {
            return RemoveLeafCase::Shrink { parent: p_id };
        }

        let sibling = self.nodes.sibling_of(p_id, v_id).0;
        let grandparent = p.parent();
        RemoveLeafCase::Collapse {
            parent: p_id,
            sibling,
            grandparent,
        }
    }

    fn remove_root_leaf(&mut self, v_id: VNodeId) {
        self.nodes.dealloc(v_id.index());
        self.nodes.root = None;
    }

    fn remove_from_three_child_parent(&mut self, p_id: VNodeId, v_id: VNodeId) {
        self.remove_structural_child(p_id, v_id);
        self.finalize_after_parent_change(p_id);
        self.nodes.dealloc(v_id.index());
    }

    fn collapse_two_child_parent(
        &mut self,
        p_id: VNodeId,
        sole_id: VNodeId,
        grandparent: Option<VNodeId>,
        v_id: VNodeId,
    ) {
        self.nodes.get_mut(sole_id.index()).set_parent_opt(grandparent);

        match grandparent {
            None => self.nodes.root = Some(sole_id),
            Some(g_id) => {
                let sole_int = self.nodes.get(sole_id.index()).intensity();
                self.replace_structural_child(g_id, p_id, sole_id, sole_int);
                self.finalize_after_parent_change(g_id);
            }
        }

        self.nodes.dealloc(p_id.index());
        self.nodes.dealloc(v_id.index());
    }

    fn finalize_after_parent_change(&mut self, parent_id: VNodeId) {
        self.recompute_and_propagate_v_sums(parent_id);
        self.propagate_evictable(parent_id);
    }

    // ── Sum / intensity propagation ───────────────────────────────────────

    pub(crate) fn propagate_sums(&mut self, id: VNodeId) {
        self.propagate_v_sums(id);
    }

    pub(crate) fn sync_intensity(&mut self, id: VNodeId, val: V) {
        self.sync_intensity_in_parent(id, val);
    }

    pub(crate) fn recompute_all_intensities(&mut self) {
        if let Some(root) = self.nodes.root {
            self.recompute_all_v_intensities(root);
        }
    }

    // ── Depth cache ───────────────────────────────────────────────────────

    pub(crate) fn depth(&self, id: VNodeId) -> u32 {
        self.nodes.depth(id)
    }

    // ── Evictable flags ───────────────────────────────────────────────────

    pub(crate) fn propagate_evictable(&mut self, id: VNodeId) {
        self.propagate_evictable_flags(id);
    }

    pub(crate) fn set_entry_flags(
        &mut self,
        entry_id: VNodeId,
        is_exposed_value: bool,
        is_evictable_value: bool,
    ) {
        self.nodes
            .set_entry_flags(entry_id, is_exposed_value, is_evictable_value);
    }

    pub(crate) fn add_structural_child(&mut self, parent: VNodeId, child: VNodeId, intensity: V) {
        self.nodes.add_structural_child(parent, child, intensity);
    }

    pub(crate) fn replace_structural_child(
        &mut self,
        parent: VNodeId,
        old_child: VNodeId,
        new_child: VNodeId,
        new_intensity: V,
    ) {
        self.nodes
            .replace_structural_child(parent, old_child, new_child, new_intensity);
    }

    pub(crate) fn remove_structural_child(&mut self, parent: VNodeId, child: VNodeId) {
        self.nodes.remove_structural_child(parent, child);
    }

    pub(crate) fn recompute_and_sync(&mut self, id: VNodeId) {
        self.recompute_and_sync_parent_slot(id);
    }

    /// Allocates a new 2-child structural node whose children are `a` and `b`,
    /// linking both children back to the new node. Returns the new node's id.
    pub(crate) fn alloc_structural_2(&mut self, a: VNodeId, b: VNodeId) -> VNodeId {
        self.nodes.alloc_structural_2(a, b)
    }

    // ── Eviction candidate scan ───────────────────────────────────────────

    /// Returns all V-entry nodes eligible for eviction.
    ///
    /// A node is a candidate when it is deeper than `live_depth_evict`,
    /// flagged `is_evictable`, and does not belong to the G-tree root.
    pub(crate) fn scan_for_candidates(
        &self,
        live_depth_evict: u32,
        g_root: GNodeId,
    ) -> Vec<VNodeId> {
        self.nodes.scan_for_candidates(live_depth_evict, g_root)
    }

    // ── Internal arena-based helpers ──────────────────────────────────────

    fn recompute_and_propagate_v_sums(&mut self, start: VNodeId) {
        self.nodes.recompute_and_propagate_v_sums(start);
    }

    /// Recomputes the structural intensity of `child_id` and immediately
    /// updates the cached intensity slot in its parent (if any).
    fn recompute_and_sync_parent_slot(&mut self, child_id: VNodeId) {
        self.recompute_structural_intensity(child_id);
        let new_int = self.nodes.get(child_id.index()).intensity();
        self.sync_intensity_in_parent(child_id, new_int);
    }

    /// Walks ancestors of `start` (exclusive — `start` itself is not
    /// recomputed) and updates each structural node's intensity and its cached
    /// slot in its parent.
    fn propagate_v_sums(&mut self, start: VNodeId) {
        self.nodes.propagate_v_sums(start);
    }

    /// Recomputes intensities for every node in the tree rooted at `v_root`
    /// (full post-order traversal).
    fn recompute_all_v_intensities(&mut self, v_root: VNodeId) {
        self.nodes.recompute_all_v_intensities(v_root);
    }

    fn propagate_evictable_flags(&mut self, start: VNodeId) {
        self.nodes.propagate_evictable_flags(start);
    }

    fn sync_intensity_in_parent(&mut self, child_id: VNodeId, new_intensity: V) {
        self.nodes.sync_intensity_in_parent(child_id, new_intensity);
    }

    fn recompute_structural_intensity(&mut self, id: VNodeId) {
        self.nodes.recompute_structural_intensity(id);
    }
}

#[cfg(test)]
mod tests {
    use super::{VNodeTree, VTree};
    use crate::arena::Arena;
    use crate::handle::{GNodeId, VNodeId};
    use crate::nodes::gnode::GNode;
    use crate::nodes::vnode::{Children, VKind, VNode};

    fn entry_vnode(intensity: u32, parent: Option<VNodeId>) -> VNode<u32> {
        VNode::new_entry(intensity, parent, GNodeId::from_index(0), true, true)
    }

    fn entry_vnode_for(gnode: GNodeId, intensity: u32, parent: Option<VNodeId>) -> VNode<u32> {
        VNode::new_entry(intensity, parent, gnode, true, true)
    }

    // ── v_depth ──────────────────────────────────────────────────────────
    mod v_depth_fn {
        use super::*;

        #[test]
        fn root_node_has_depth_zero() {
            let mut vnodes = VNodeTree::from(Arena::new());
            let id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)).0);
            assert_eq!(vnodes.depth(id), 0);
        }

        #[test]
        fn child_of_root_has_depth_one() {
            let mut vnodes = VNodeTree::from(Arena::new());
            let root_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)).0);
            let child_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, Some(root_id))).0);
            assert_eq!(vnodes.depth(child_id), 1);
        }

        #[test]
        fn depth_is_consistent_across_calls() {
            let mut vnodes = VNodeTree::from(Arena::new());
            let id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)).0);
            let d1 = vnodes.depth(id);
            let d2 = vnodes.depth(id);
            assert_eq!(d1, d2);
        }
    }

    // ── propagate_v_sums ─────────────────────────────────────────────────
    mod propagate_v_sums_fn {
        use super::*;

        #[test]
        fn propagating_from_root_entry_does_not_panic() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let root_id = VNodeId::from_index(vnodes.alloc(entry_vnode(10, None)).0);
            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(root_id);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };
            // Root has no parent; propagate_v_sums is a no-op but must not panic
            vtree.propagate_sums(root_id);
        }

        #[test]
        fn propagating_from_child_updates_structural_parent() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();

            // Allocate two placeholder slots to get stable IDs
            let child_a_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)).0);
            let child_b_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)).0);

            let parent_node: VNode<u32> = VNode::new_structural(
                0,
                None,
                Children::new_2((child_a_id, 5u32), (child_b_id, 7u32)),
                false,
            );
            let parent_id = VNodeId::from_index(vnodes.alloc(parent_node).0);

            // Wire children back to parent
            vnodes.get_mut(child_a_id.index()).set_parent(parent_id);
            vnodes.get_mut(child_b_id.index()).set_parent(parent_id);

            // Update child_a's own intensity
            vnodes.get_mut(child_a_id.index()).set_intensity(20);

            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(parent_id);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };
            vtree.propagate_sums(child_a_id);

            // Parent intensity should now reflect sum of cached child intensities
            // (The structural node caches 5 and 7; propagate_v_sums recomputes from them)
            let parent_intensity = vtree.nodes.get(parent_id.index()).intensity();
            assert_eq!(parent_intensity, 5 + 7); // cached intensities in Children
        }
    }

    // ── vtree_remove_leaf ─────────────────────────────────────────────────
    mod vtree_remove_leaf_fn {
        use super::*;
        use crate::tree::gtree::{GNodeTree, GTree};

        /// Minimal G-tree with one Terminal `GNode` at index 0.
        fn gtree_with_one_node() -> GTree<u8, u32, 8> {
            let mut gnodes: crate::arena::Arena<GNode<u8, u32>> = Arena::new();
            let root = GNodeId::from_index(gnodes.alloc(GNode::new_leaf(0u8, 255u8, 0u32, None)).0);
            GTree {
                nodes: GNodeTree {
                    nodes: gnodes,
                    root,
                    node_count: 1,
                    terminal_count: 1,
                },
                live_depth_evict: 5,
                live_depth_create: 3,
                depth_buffer: 2,
                headroom: 1,
                soft_limit: None,
            }
        }

        // Root-removal: node has no parent → returns None.
        #[test]
        fn root_removal_returns_none() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let mut gtree = gtree_with_one_node();
            let v_root = VNodeId::from_index(vnodes.alloc(entry_vnode(10, None)).0);
            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(v_root);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };
            vtree.remove_leaf(&mut gtree, v_root);
            assert!(vtree.nodes.root.is_none());
            assert!(!vtree.nodes.is_occupied(v_root.index()));
        }

        #[test]
        fn root_removal_clears_gnode_entry_link() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let mut gtree = gtree_with_one_node();
            let g_root = gtree.nodes.root;
            let v_root = VNodeId::from_index(vnodes.alloc(entry_vnode_for(g_root, 10, None)).0);

            gtree.nodes.assign_entry(g_root, v_root);
            assert_eq!(gtree.nodes.get(g_root.index()).entry(), Some(v_root));

            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(v_root);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };

            vtree.remove_leaf(&mut gtree, v_root);
            assert_eq!(gtree.nodes.get(g_root.index()).entry(), None);
        }

        // Shrink case: parent has 3 children → remove one, parent shrinks to 2.
        #[test]
        fn shrink_removes_child_from_three_child_parent() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let mut gtree = gtree_with_one_node();

            let child_a = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);
            let child_b = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);
            let child_c = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);

            let parent_id = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                15,
                None,
                Children::new_3((child_a, 5u32), (child_b, 5u32), (child_c, 5u32)),
                true,
            )).0);

            vnodes.get_mut(child_a.index()).set_parent(parent_id);
            vnodes.get_mut(child_b.index()).set_parent(parent_id);
            vnodes.get_mut(child_c.index()).set_parent(parent_id);

            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(parent_id);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };

            vtree.remove_leaf(&mut gtree, child_c);
            assert_eq!(vtree.nodes.root, Some(parent_id)); // parent remains root
            assert!(!vtree.nodes.is_occupied(child_c.index())); // target removed
            match &vtree.nodes.get(parent_id.index()).kind() {
                VKind::Structural { children, .. } => assert_eq!(children.len(), 2),
                _ => panic!("expected Structural"),
            }
        }

        // Collapse / no-grandparent: 2-child parent is root; sole sibling becomes new root.
        #[test]
        fn collapse_no_grandparent_sole_sibling_becomes_root() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let mut gtree = gtree_with_one_node();

            let target = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);
            let sibling = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);

            let parent_id = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                10,
                None,
                Children::new_2((target, 5u32), (sibling, 5u32)),
                true,
            )).0);

            vnodes.get_mut(target.index()).set_parent(parent_id);
            vnodes.get_mut(sibling.index()).set_parent(parent_id);

            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(parent_id);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };

            vtree.remove_leaf(&mut gtree, target);
            assert_eq!(vtree.nodes.root, Some(sibling)); // sibling is new root
            assert!(!vtree.nodes.is_occupied(target.index()));
            assert!(!vtree.nodes.is_occupied(parent_id.index())); // parent collapsed
            assert!(vtree.nodes.get(sibling.index()).parent().is_none());
        }

        // Collapse / with grandparent: parent is removed and sibling is linked into grandparent.
        #[test]
        fn collapse_with_grandparent_replaces_parent_in_grandparent() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let mut gtree = gtree_with_one_node();

            let target = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);
            let sibling = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)).0);
            let uncle = VNodeId::from_index(vnodes.alloc(entry_vnode(4, None)).0);

            let parent_id = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                10,
                None,
                Children::new_2((target, 5u32), (sibling, 5u32)),
                true,
            )).0);

            let grandparent_id = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                14,
                None,
                Children::new_2((parent_id, 10u32), (uncle, 4u32)),
                true,
            )).0);

            vnodes.get_mut(target.index()).set_parent(parent_id);
            vnodes.get_mut(sibling.index()).set_parent(parent_id);
            vnodes.get_mut(uncle.index()).set_parent(grandparent_id);
            vnodes.get_mut(parent_id.index()).set_parent(grandparent_id);

            let mut vnode_tree = VNodeTree::from(vnodes);
            vnode_tree.root = Some(grandparent_id);
            let mut vtree = VTree {
                nodes: vnode_tree,
                violations: Vec::new(),
            };

            vtree.remove_leaf(&mut gtree, target);

            assert_eq!(vtree.nodes.root, Some(grandparent_id));
            assert!(!vtree.nodes.is_occupied(target.index()));
            assert!(!vtree.nodes.is_occupied(parent_id.index()));
            assert_eq!(vtree.nodes.get(sibling.index()).parent(), Some(grandparent_id));

            match vtree.nodes.get(grandparent_id.index()).kind() {
                VKind::Structural { children, .. } => {
                    assert_eq!(children.len(), 2);
                    let child_ids: Vec<_> = children.iter().map(|(id, _)| id).collect();
                    assert!(child_ids.contains(&sibling));
                    assert!(child_ids.contains(&uncle));
                    assert!(!child_ids.contains(&parent_id));
                }
                _ => panic!("expected Structural"),
            }
        }
    }
}
