use super::vnode::{Children, VKind, VNode};
use crate::arena::Arena;
use crate::handle::{GNodeId, VNodeId};
use crate::traits::Accumulator;
use std::ops::{Deref, DerefMut};

/// Structural V-node owner: backing storage and root identity.
///
/// All fields here depend only on the node set itself. Tree-level policy
/// parameters (eviction depth, violations queue, etc.) live on [`super::VTree`].
#[derive(Debug, Clone)]
pub struct VNodeTree<V: Accumulator> {
    /// Backing store for all V-nodes.
    nodes: Arena<VNode<V>>,
    /// Root V-node (`None` only when the tree is empty).
    pub(crate) root: Option<VNodeId>,
}

impl<V: Accumulator> VNodeTree<V> {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            nodes: Arena::new(),
            root: None,
        }
    }
}

impl<V: Accumulator> Default for VNodeTree<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Accumulator> From<Arena<VNode<V>>> for VNodeTree<V> {
    fn from(nodes: Arena<VNode<V>>) -> Self {
        Self { nodes, root: None }
    }
}

impl<V: Accumulator> Deref for VNodeTree<V> {
    type Target = Arena<VNode<V>>;

    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl<V: Accumulator> DerefMut for VNodeTree<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

impl<V: Accumulator> VNodeTree<V> {
    #[must_use]
    pub(crate) fn depth(&self, id: VNodeId) -> u32 {
        let node = self.get(id.index());
        node.parent().map_or(0, |p| self.depth(p) + 1)
    }

    pub(crate) fn compute_has_evictable(&self, children: &Children<V>) -> bool {
        for i in 0..children.len() {
            let (child_id, _) = children.get(i);
            let child = self.get(child_id.index());
            let child_flag = match &child.kind() {
                VKind::Entry { is_evictable, .. } => *is_evictable,
                VKind::Structural { has_evictable, .. } => *has_evictable,
            };
            if child_flag {
                return true;
            }
        }

        false
    }

    #[must_use]
    pub(crate) fn node_has_evictable(&self, id: VNodeId) -> bool {
        match &self.get(id.index()).kind() {
            VKind::Entry { is_evictable, .. } => *is_evictable,
            VKind::Structural { has_evictable, .. } => *has_evictable,
        }
    }

    #[must_use]
    pub(crate) fn sibling_of(&self, parent: VNodeId, child: VNodeId) -> (VNodeId, V) {
        let p_node = self.get(parent.index());
        match &p_node.kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(
                    children.len(),
                    2,
                    "sibling_of: parent must be a 2-child structural node"
                );

                let (id0, int0) = children.get(0);
                let (id1, int1) = children.get(1);

                if id0 == child {
                    (id1, int1)
                } else if id1 == child {
                    (id0, int0)
                } else {
                    panic!(
                        "sibling_of: child {} not found in parent {}",
                        child.index(),
                        parent.index()
                    );
                }
            }
            VKind::Entry { .. } => panic!("sibling_of: parent must be structural"),
        }
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
        } = self.get_mut(entry_id.index()).kind_mut()
        {
            *is_exposed = is_exposed_value;
            *is_evictable = is_evictable_value;
        }
    }

    pub(crate) fn add_structural_child(&mut self, parent: VNodeId, child: VNodeId, intensity: V) {
        let p = self.get_mut(parent.index());
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
        let p = self.get_mut(parent.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            children.replace_child(old_child, new_child, new_intensity);
        }
    }

    pub(crate) fn remove_structural_child(&mut self, parent: VNodeId, child: VNodeId) {
        let p = self.get_mut(parent.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            children.remove_child(child);
        }
    }

    /// Walks ancestors of `start` (exclusive — `start` itself is not
    /// recomputed) and updates each structural node's intensity and its cached
    /// slot in its parent.
    pub(crate) fn propagate_v_sums(&mut self, start: VNodeId) {
        tracing::trace!(start = start.index(), "propagate_v_sums");
        let mut current = self.get(start.index()).parent();
        while let Some(id) = current {
            self.recompute_structural_intensity(id);
            let new_int = self.get(id.index()).intensity();
            self.sync_intensity_in_parent(id, new_int);
            current = self.get(id.index()).parent();
        }
    }

    /// Recomputes intensities for every node in the tree rooted at `v_root`
    /// (full post-order traversal).
    pub(crate) fn recompute_all_v_intensities(&mut self, v_root: VNodeId) {
        self.recompute_v_postorder(v_root);
    }

    fn recompute_v_postorder(&mut self, id: VNodeId) {
        // Collect child IDs while holding a shared borrow, then release it so
        // the recursive calls and the subsequent mutable borrows can proceed.
        let child_ids: Vec<VNodeId> = {
            let node = self.get(id.index());
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
            let child_int = self.get(child.index()).intensity();
            let node = self.get_mut(id.index());
            if let VKind::Structural { children, .. } = node.kind_mut() {
                children.update_intensity(i, child_int);
            }
        }

        // Sum updated child intensities. The shared borrow must end
        // (NLL last-use) before the mutable borrow on the next line, so total
        // is computed first.
        let total: V = {
            let node = self.get(id.index());
            let VKind::Structural { children, .. } = &node.kind() else {
                return;
            };
            let mut t = V::zero();
            for i in 0..children.len() {
                t = V::add(t, children.get(i).1);
            }
            t
        };
        self.get_mut(id.index()).set_intensity(total);
    }

    pub(crate) fn propagate_evictable_flags(&mut self, start: VNodeId) {
        let mut current = Some(start);
        while let Some(id) = current {
            let node = self.get(id.index());
            match &node.kind() {
                VKind::Entry { .. } => {
                    current = node.parent();
                }
                VKind::Structural {
                    children,
                    has_evictable,
                } => {
                    let old = *has_evictable;
                    let new_flag = self.compute_has_evictable(children);
                    if new_flag == old {
                        return;
                    }

                    let parent = node.parent();
                    let node_mut = self.get_mut(id.index());
                    if let VKind::Structural { has_evictable, .. } = node_mut.kind_mut() {
                        *has_evictable = new_flag;
                    }
                    current = parent;
                }
            }
        }
    }

    pub(crate) fn sync_intensity_in_parent(&mut self, child_id: VNodeId, new_intensity: V) {
        let parent = self.get(child_id.index()).parent();
        let Some(p_id) = parent else { return };
        let p = self.get_mut(p_id.index());
        if let VKind::Structural { children, .. } = p.kind_mut() {
            if let Some(idx) = children.find_index(child_id) {
                children.update_intensity(idx, new_intensity);
            }
        }
    }

    pub(crate) fn recompute_structural_intensity(&mut self, id: VNodeId) {
        let node = self.get(id.index());
        if let VKind::Structural { children, .. } = &node.kind() {
            let mut total = V::zero();
            for i in 0..children.len() {
                total = V::add(total, children.get(i).1);
            }

            let _ = node;
            self.get_mut(id.index()).set_intensity(total);
        }
    }

    /// Recomputes the structural intensity of `child_id` and immediately
    /// updates the cached intensity slot in its parent (if any).
    pub(crate) fn recompute_and_sync_parent_slot(&mut self, child_id: VNodeId) {
        self.recompute_structural_intensity(child_id);
        let new_int = self.get(child_id.index()).intensity();
        self.sync_intensity_in_parent(child_id, new_int);
    }

    pub(crate) fn recompute_and_propagate_v_sums(&mut self, start: VNodeId) {
        self.recompute_and_sync_parent_slot(start);
        self.propagate_v_sums(start);
    }

    /// Allocates a new 2-child structural node whose children are `a` and `b`,
    /// linking both children back to the new node. Returns the new node's id.
    pub(crate) fn alloc_structural_2(&mut self, a: VNodeId, b: VNodeId) -> VNodeId {
        let a_int = self.get(a.index()).intensity();
        let b_int = self.get(b.index()).intensity();
        let s = VNode::new_structural(
            V::add(a_int, b_int),
            None,
            Children::new_2((a, a_int), (b, b_int)),
            true,
        );
        let s_id = VNodeId::from_index(self.alloc(s).0);
        self.get_mut(a.index()).set_parent(s_id);
        self.get_mut(b.index()).set_parent(s_id);
        s_id
    }

    #[must_use]
    pub(crate) fn max_uncle_intensity(&self, c: VNodeId) -> Option<V> {
        let parent = self.get(c.index()).parent()?;
        let grandparent = self.get(parent.index()).parent()?;

        let g = self.get(grandparent.index());
        if let VKind::Structural { children, .. } = &g.kind() {
            let mut max_int = None;
            for i in 0..children.len() {
                let (id, intensity) = children.get(i);
                if id != parent {
                    max_int = Some(max_int.map_or(intensity, |cur| {
                        if intensity > cur { intensity } else { cur }
                    }));
                }
            }
            max_int
        } else {
            None
        }
    }

    #[must_use]
    pub(crate) fn is_violated(&self, c: VNodeId) -> bool {
        let c_int = self.get(c.index()).intensity();
        self.max_uncle_intensity(c)
            .is_some_and(|max_uncle| c_int > max_uncle)
    }

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
        let node = self.get(v_id.index());
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
    use super::*;

    fn make_tree() -> VNodeTree<u32> {
        VNodeTree::from(Arena::new())
    }

    #[test]
    fn new_starts_with_empty_arena_and_no_root() {
        let t = VNodeTree::<u32>::new();
        assert_eq!(t.count(), 0);
        assert!(t.root.is_none());
    }

    #[test]
    fn default_starts_with_empty_arena_and_no_root() {
        let t: VNodeTree<u32> = VNodeTree::default();
        assert_eq!(t.count(), 0);
        assert!(t.root.is_none());
    }

    #[test]
    #[should_panic(expected = "sibling_of: parent must be structural")]
    fn sibling_of_panics_when_parent_is_entry() {
        let mut t = make_tree();
        let parent = VNodeId::from_index(
            t.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(0),
                true,
                true,
            ))
            .0,
        );
        let child = VNodeId::from_index(
            t.alloc(VNode::new_entry(
                2,
                None,
                GNodeId::from_index(1),
                true,
                true,
            ))
            .0,
        );
        let _ = t.sibling_of(parent, child);
    }

    #[test]
    #[should_panic(expected = "sibling_of: child")]
    fn sibling_of_panics_when_child_is_not_in_parent() {
        let mut t = make_tree();
        let c1 = VNodeId::from_index(
            t.alloc(VNode::new_entry(
                3,
                None,
                GNodeId::from_index(2),
                true,
                true,
            ))
            .0,
        );
        let c2 = VNodeId::from_index(
            t.alloc(VNode::new_entry(
                4,
                None,
                GNodeId::from_index(3),
                true,
                true,
            ))
            .0,
        );
        let outsider = VNodeId::from_index(
            t.alloc(VNode::new_entry(
                5,
                None,
                GNodeId::from_index(4),
                true,
                true,
            ))
            .0,
        );
        let p = VNodeId::from_index(
            t.alloc(VNode::new_structural(
                7,
                None,
                Children::new_2((c1, 3), (c2, 4)),
                true,
            ))
            .0,
        );
        t.get_mut(c1.index()).set_parent(p);
        t.get_mut(c2.index()).set_parent(p);
        let _ = t.sibling_of(p, outsider);
    }

    // ── max_uncle_intensity ───────────────────────────────────────────

    #[test]
    fn max_uncle_intensity_returns_max_across_multiple_uncles() {
        let mut vnodes = VNodeTree::<u32>::from(Arena::new());

        let c = VNodeId::from_index(
            vnodes
                .alloc(VNode::new_entry(
                    5,
                    None,
                    GNodeId::from_index(1),
                    true,
                    true,
                ))
                .0,
        );
        let sibling = VNodeId::from_index(
            vnodes
                .alloc(VNode::new_entry(
                    4,
                    None,
                    GNodeId::from_index(2),
                    true,
                    true,
                ))
                .0,
        );
        let u1 = VNodeId::from_index(
            vnodes
                .alloc(VNode::new_entry(
                    9,
                    None,
                    GNodeId::from_index(3),
                    true,
                    true,
                ))
                .0,
        );

        let parent = VNodeId::from_index(
            vnodes
                .alloc(VNode::new_structural(
                    9,
                    None,
                    Children::new_2((c, 5), (sibling, 4)),
                    true,
                ))
                .0,
        );
        let gp = VNodeId::from_index(
            vnodes
                .alloc(VNode::new_structural(
                    18,
                    None,
                    Children::new_2((parent, 9), (u1, 9)),
                    true,
                ))
                .0,
        );

        vnodes.get_mut(c.index()).set_parent(parent);
        vnodes.get_mut(sibling.index()).set_parent(parent);
        vnodes.get_mut(parent.index()).set_parent(gp);
        vnodes.get_mut(u1.index()).set_parent(gp);

        assert_eq!(vnodes.max_uncle_intensity(c), Some(9));
    }
}
