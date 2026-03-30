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
use crate::arena::Arena;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::{Children, VKind, VNode};
use crate::tree::gtree::GTree;
use crate::traits::{Accumulator, Coordinate};

mod traversal;
use traversal::compute_has_evictable;
use traversal::v_depth;


// ── VTree ────────────────────────────────────────────────────────────────────

/// The V-tree: an intensity-aggregation binary tree overlaid on the G-tree.
/// Owns the node arena, the root pointer, and the violations queue.
#[derive(Debug, Clone)]
pub struct VTree<V: Accumulator> {
    /// Backing store for all V-nodes.
    pub(crate) nodes: Arena<VNode<V>>,
    /// Root V-node (`None` only when the tree is empty).
    pub(crate) root: Option<VNodeId>,
    /// V-nodes whose intensity distribution violates the balance threshold.
    pub(crate) violations: Vec<VNodeId>,
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
            gtree.clear_entry(*gnode);
        }

        let parent = self.nodes.get(v_id.index()).parent();

        let Some(p_id) = parent else {
            span.record("case", "root");
            self.nodes.dealloc(v_id.index());
            self.root = None;
            return;
        };

        let p = self.nodes.get(p_id.index());
        let p_child_count = match &p.kind() {
            VKind::Structural { children, .. } => children.len(),
            VKind::Entry { .. } => unreachable!("parent of entry should be structural"),
        };

        if p_child_count == 3 {
            span.record("case", "shrink");
            self.remove_structural_child(p_id, v_id);
            self.recompute_and_propagate_v_sums(p_id);
            self.propagate_evictable(p_id);
            self.nodes.dealloc(v_id.index());
            return;
        }

        span.record("case", "collapse");
        let sole_id = self.sole_sibling(p_id, v_id);
        let grandparent = self.nodes.get(p_id.index()).parent();

        self.nodes.get_mut(sole_id.index()).set_parent_opt(grandparent);

        self.root = grandparent.map_or(Some(sole_id), |g_id| {
            let sole_int = self.nodes.get(sole_id.index()).intensity();
            self.replace_structural_child(g_id, p_id, sole_id, sole_int);
            self.recompute_and_propagate_v_sums(g_id);
            self.propagate_evictable(g_id);
            self.root
        });

        self.nodes.dealloc(p_id.index());
        self.nodes.dealloc(v_id.index());
    }

    // ── Sum / intensity propagation ───────────────────────────────────────

    pub(crate) fn propagate_sums(&mut self, id: VNodeId) {
        self.propagate_v_sums(id);
    }

    pub(crate) fn sync_intensity(&mut self, id: VNodeId, val: V) {
        self.sync_intensity_in_parent(id, val);
    }

    pub(crate) fn recompute_all_intensities(&mut self) {
        if let Some(root) = self.root {
            self.recompute_all_v_intensities(root);
        }
    }

    // ── Depth cache ───────────────────────────────────────────────────────

    pub(crate) fn depth(&self, id: VNodeId) -> u32 {
        v_depth(&self.nodes, id)
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
        if let VKind::Entry {
            is_exposed,
            is_evictable,
            ..
        } = self.nodes.get_mut(entry_id.index()).kind_mut()
        {
            *is_exposed = is_exposed_value;
            *is_evictable = is_evictable_value;
        }
    }

    pub(crate) fn add_structural_child(&mut self, parent: VNodeId, child: VNodeId, intensity: V) {
        let p = self.nodes.get_mut(parent.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            children.add_child(child, intensity);
        }
    }

    pub(crate) fn replace_structural_child(
        &mut self,
        parent: VNodeId,
        old_child: VNodeId,
        new_child: VNodeId,
        new_intensity: V,
    ) {
        let p = self.nodes.get_mut(parent.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            children.replace_child(old_child, new_child, new_intensity);
        }
    }

    pub(crate) fn remove_structural_child(&mut self, parent: VNodeId, child: VNodeId) {
        let p = self.nodes.get_mut(parent.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            children.remove_child(child);
        }
    }

    pub(crate) fn recompute_and_sync(&mut self, id: VNodeId) {
        self.recompute_and_sync_parent_slot(id);
    }

    // ── Internal arena-based helpers ──────────────────────────────────────

    /// Walks ancestors of `start` (exclusive — `start` itself is not
    /// recomputed) and updates each structural node's intensity and its cached
    /// slot in its parent.
    fn propagate_v_sums(&mut self, start: VNodeId) {
        tracing::trace!(start = start.index(), "propagate_v_sums");
        let mut current = self.nodes.get(start.index()).parent();
        while let Some(id) = current {
            self.recompute_structural_intensity(id);
            let new_int = self.nodes.get(id.index()).intensity();
            self.sync_intensity_in_parent(id, new_int);
            current = self.nodes.get(id.index()).parent();
        }
    }

    /// Recomputes intensities for every node in the tree rooted at `v_root`
    /// (full post-order traversal).
    fn recompute_all_v_intensities(&mut self, v_root: VNodeId) {
        self.recompute_v_postorder(v_root);
    }

    fn recompute_v_postorder(&mut self, id: VNodeId) {
        // Collect child IDs while holding a shared borrow, then release it so
        // the recursive calls and the subsequent mutable borrows can proceed.
        let child_ids: Vec<VNodeId> = {
            let node = self.nodes.get(id.index());
            match &node.kind() {
                VKind::Entry { .. } => return,
                VKind::Structural { children, .. } => {
                    (0..children.len()).map(|i| children.get(i).0).collect()
                }
            }
        };

        for &child in &child_ids {
            self.recompute_v_postorder(child);
        }

        for (i, &child) in child_ids.iter().enumerate() {
            let child_int = self.nodes.get(child.index()).intensity();
            let node = self.nodes.get_mut(id.index());
            if let VKind::Structural { children, .. } = node.kind_mut() {
                children.update_intensity(i, child_int);
            }
        }

        // Sum updated child intensities. The shared borrow must end
        // (NLL last-use) before the mutable borrow on the next line, so total
        // is computed first.
        let total: V = {
            let node = self.nodes.get(id.index());
            let VKind::Structural { children, .. } = &node.kind() else {
                return;
            };
            let mut t = V::zero();
            for i in 0..children.len() {
                t = V::add(t, children.get(i).1);
            }
            t
        };
        self.nodes.get_mut(id.index()).set_intensity(total);
    }

    fn propagate_evictable_flags(&mut self, start: VNodeId) {
        let mut current = Some(start);
        while let Some(id) = current {
            let node = self.nodes.get(id.index());
            match &node.kind() {
                VKind::Entry { .. } => {
                    current = node.parent();
                }
                VKind::Structural {
                    children,
                    has_evictable,
                } => {
                    let old = *has_evictable;
                    let new_flag = compute_has_evictable(&self.nodes, children);
                    if new_flag == old {
                        return;
                    }

                    let parent = node.parent();
                    let node_mut = self.nodes.get_mut(id.index());
                    if let VKind::Structural { has_evictable, .. } = node_mut.kind_mut() {
                        *has_evictable = new_flag;
                    }
                    current = parent;
                }
            }
        }
    }

    fn sync_intensity_in_parent(&mut self, child_id: VNodeId, new_intensity: V) {
        let parent = self.nodes.get(child_id.index()).parent();
        let Some(p_id) = parent else { return };
        let p = self.nodes.get_mut(p_id.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            if let Some(idx) = children.find_index(child_id) {
                children.update_intensity(idx, new_intensity);
            }
        }
    }

    fn recompute_structural_intensity(&mut self, id: VNodeId) {
        let node = self.nodes.get(id.index());
        if let VKind::Structural { children, .. } = &node.kind() {
            let mut total = V::zero();
            for i in 0..children.len() {
                total = V::add(total, children.get(i).1);
            }

            let _ = node;
            self.nodes.get_mut(id.index()).set_intensity(total);
        }
    }

    /// Recomputes the structural intensity of `child_id` and immediately
    /// updates the cached intensity slot in its parent (if any).
    fn recompute_and_sync_parent_slot(&mut self, child_id: VNodeId) {
        self.recompute_structural_intensity(child_id);
        let new_int = self.nodes.get(child_id.index()).intensity();
        self.sync_intensity_in_parent(child_id, new_int);
    }

    fn recompute_and_propagate_v_sums(&mut self, start: VNodeId) {
        self.recompute_and_sync_parent_slot(start);
        self.propagate_v_sums(start);
    }

    fn sole_sibling(&self, parent: VNodeId, child: VNodeId) -> VNodeId {
        let p = self.nodes.get(parent.index());
        if let VKind::Structural { children, .. } = &p.kind() {
            for i in 0..children.len() {
                let (id, _) = children.get(i);
                if id != child {
                    return id;
                }
            }
        }
        unreachable!("sole_sibling: child not found in parent");
    }

    /// Allocates a new 2-child structural node whose children are `a` and `b`,
    /// linking both children back to the new node. Returns the new node's id.
    pub(crate) fn alloc_structural_2(&mut self, a: VNodeId, b: VNodeId) -> VNodeId {
        let a_int = self.nodes.get(a.index()).intensity();
        let b_int = self.nodes.get(b.index()).intensity();
        let s = VNode::new_structural(
            V::add(a_int, b_int),
            None,
            Children::new_2((a, a_int), (b, b_int)),
            true,
        );
        let s_id = VNodeId::from_index(self.nodes.alloc(s));
        self.nodes.get_mut(a.index()).set_parent(s_id);
        self.nodes.get_mut(b.index()).set_parent(s_id);
        s_id
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
        let _span = tracing::trace_span!("scan_for_candidates").entered();
        let mut candidates = Vec::new();
        if let Some(v_root) = self.root {
            self.scan_dfs(v_root, 0, live_depth_evict, g_root, &mut candidates);
        }
        tracing::trace!(candidates = candidates.len(), "scan complete");
        candidates
    }

    fn scan_dfs(
        &self,
        v_id: VNodeId,
        depth: u32,
        live_depth_evict: u32,
        g_root: GNodeId,
        candidates: &mut Vec<VNodeId>,
    ) {
        let node = self.nodes.get(v_id.index());
        match &node.kind() {
            VKind::Entry {
                gnode,
                is_evictable,
                ..
            } => {
                if depth > live_depth_evict && *is_evictable && *gnode != g_root {
                    candidates.push(v_id);
                }
            }
            VKind::Structural {
                children,
                has_evictable,
            } => {
                if !has_evictable {
                    return;
                }
                for i in 0..children.len() {
                    let (child_id, _) = children.get(i);
                    self.scan_dfs(child_id, depth + 1, live_depth_evict, g_root, candidates);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::{VTree, v_depth};
    use crate::arena::Arena;
    use crate::handle::{GNodeId, VNodeId};
    use crate::nodes::vnode::{Children, VKind, VNode};

    fn entry_vnode(intensity: u32, parent: Option<VNodeId>) -> VNode<u32> {
        VNode::new_entry(intensity, parent, GNodeId::from_index(0), true, true)
    }

    // ── v_depth ──────────────────────────────────────────────────────────
    mod v_depth_fn {
        use super::*;

        #[test]
        fn root_node_has_depth_zero() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)));
            assert_eq!(v_depth(&vnodes, id), 0);
        }

        #[test]
        fn child_of_root_has_depth_one() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let root_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)));
            let child_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, Some(root_id))));
            assert_eq!(v_depth(&vnodes, child_id), 1);
        }

        #[test]
        fn depth_is_consistent_across_calls() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)));
            let d1 = v_depth(&vnodes, id);
            let d2 = v_depth(&vnodes, id);
            assert_eq!(d1, d2);
        }
    }

    // ── propagate_v_sums ─────────────────────────────────────────────────
    mod propagate_v_sums_fn {
        use super::*;

        #[test]
        fn propagating_from_root_entry_does_not_panic() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let root_id = VNodeId::from_index(vnodes.alloc(entry_vnode(10, None)));
            let mut vtree = VTree {
                nodes: vnodes,
                root: Some(root_id),
                violations: Vec::new(),
            };
            // Root has no parent; propagate_v_sums is a no-op but must not panic
            vtree.propagate_sums(root_id);
        }

        #[test]
        fn propagating_from_child_updates_structural_parent() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();

            // Allocate two placeholder slots to get stable IDs
            let child_a_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)));
            let child_b_id = VNodeId::from_index(vnodes.alloc(entry_vnode(0, None)));

            let parent_node: VNode<u32> = VNode::new_structural(
                0,
                None,
                Children::new_2((child_a_id, 5u32), (child_b_id, 7u32)),
                false,
            );
            let parent_id = VNodeId::from_index(vnodes.alloc(parent_node));

            // Wire children back to parent
            vnodes.get_mut(child_a_id.index()).set_parent(parent_id);
            vnodes.get_mut(child_b_id.index()).set_parent(parent_id);

            // Update child_a's own intensity
            vnodes.get_mut(child_a_id.index()).set_intensity(20);

            let mut vtree = VTree {
                nodes: vnodes,
                root: Some(parent_id),
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
        use crate::nodes::gnode::GNode;
        use crate::tree::gtree::GTree;

        /// Minimal G-tree with one Terminal `GNode` at index 0.
        fn gtree_with_one_node() -> GTree<u8, u32, 8> {
            let mut gnodes: Arena<GNode<u8, u32>> = Arena::new();
            let root = GNodeId::from_index(gnodes.alloc(GNode::new_leaf(0u8, 255u8, 0u32, None)));
            GTree {
                nodes: gnodes,
                root,
                node_count: 1,
                terminal_count: 1,
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
            let v_root = VNodeId::from_index(vnodes.alloc(entry_vnode(10, None)));
            let mut vtree = VTree {
                nodes: vnodes,
                root: Some(v_root),
                violations: Vec::new(),
            };
            vtree.remove_leaf(&mut gtree, v_root);
            assert!(vtree.root.is_none());
            assert!(!vtree.nodes.is_occupied(v_root.index()));
        }

        // Shrink case: parent has 3 children → remove one, parent shrinks to 2.
        #[test]
        fn shrink_removes_child_from_three_child_parent() {
            let mut vnodes: Arena<VNode<u32>> = Arena::new();
            let mut gtree = gtree_with_one_node();

            let child_a = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)));
            let child_b = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)));
            let child_c = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)));

            let parent_id = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                15,
                None,
                Children::new_3((child_a, 5u32), (child_b, 5u32), (child_c, 5u32)),
                true,
            )));

            vnodes.get_mut(child_a.index()).set_parent(parent_id);
            vnodes.get_mut(child_b.index()).set_parent(parent_id);
            vnodes.get_mut(child_c.index()).set_parent(parent_id);

            let mut vtree = VTree {
                nodes: vnodes,
                root: Some(parent_id),
                violations: Vec::new(),
            };

            vtree.remove_leaf(&mut gtree, child_c);
            assert_eq!(vtree.root, Some(parent_id)); // parent remains root
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

            let target = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)));
            let sibling = VNodeId::from_index(vnodes.alloc(entry_vnode(5, None)));

            let parent_id = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                10,
                None,
                Children::new_2((target, 5u32), (sibling, 5u32)),
                true,
            )));

            vnodes.get_mut(target.index()).set_parent(parent_id);
            vnodes.get_mut(sibling.index()).set_parent(parent_id);

            let mut vtree = VTree {
                nodes: vnodes,
                root: Some(parent_id),
                violations: Vec::new(),
            };

            vtree.remove_leaf(&mut gtree, target);
            assert_eq!(vtree.root, Some(sibling)); // sibling is new root
            assert!(!vtree.nodes.is_occupied(target.index()));
            assert!(!vtree.nodes.is_occupied(parent_id.index())); // parent collapsed
            assert!(vtree.nodes.get(sibling.index()).parent().is_none());
        }
    }
}
