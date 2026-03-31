use super::gnode::GNode;
use crate::arena::Arena;
use crate::handle::{GNodeId, VNodeId};
use crate::traits::{Accumulator, Coordinate};

/// Structural owner of the G-node set: backing storage, root identity, and
/// intrinsic structural statistics.
///
/// All fields here depend only on the node set itself.  Tree-level policy
/// parameters (depth limits, headroom, soft limit) live on [`super::GTree`].
#[derive(Debug, Clone)]
pub struct GNodeTree<C: Coordinate, V: Accumulator> {
    /// Backing store for all G-nodes.
    pub(crate) nodes: Arena<GNode<C, V>>,
    /// Root G-node (always present).
    pub(crate) root: GNodeId,
    /// Total number of live G-nodes (terminals + internal).
    pub(crate) node_count: u32,
    /// Number of terminal (leaf) G-nodes.
    pub(crate) terminal_count: u32,
}

impl<C: Coordinate, V: Accumulator> GNodeTree<C, V> {
    // ── Arena delegation ─────────────────────────────────────────────────

    /// Returns a shared reference to the G-node at slot `idx`.
    pub(crate) fn get(&self, idx: usize) -> &GNode<C, V> {
        self.nodes.get(idx)
    }

    /// Returns a mutable reference to the G-node at slot `idx`.
    pub(crate) fn get_mut(&mut self, idx: usize) -> &mut GNode<C, V> {
        self.nodes.get_mut(idx)
    }

    /// Allocates a new G-node slot and returns its index.
    pub(crate) fn alloc(&mut self, node: GNode<C, V>) -> usize {
        self.nodes.alloc(node).0
    }

    /// Frees the G-node slot at `idx` and returns the evicted value.
    pub(crate) fn dealloc(&mut self, idx: usize) -> GNode<C, V> {
        self.nodes.dealloc(idx)
    }

    /// Returns `true` if slot `idx` holds a live G-node.
    pub(crate) fn is_occupied(&self, idx: usize) -> bool {
        self.nodes.is_occupied(idx)
    }

    /// Returns the number of currently occupied G-node slots.
    pub(crate) const fn count(&self) -> u32 {
        self.nodes.count()
    }

    /// Iterates over all occupied G-node slots as `(index, &GNode)` pairs.
    pub(crate) fn iter_occupied(&self) -> impl Iterator<Item = (usize, &GNode<C, V>)> + '_ {
        self.nodes.iter_occupied()
    }

    // ── Traversal ────────────────────────────────────────────────────────

    /// Walks down the G-tree from the root and returns the terminal (or
    /// semi-internal) G-node that *receives* coordinate `x`.
    #[must_use]
    pub(crate) fn route_to(&self, x: C) -> GNodeId {
        let mut current = self.root;
        loop {
            let g = self.nodes.get(current.index());
            let mid = C::midpoint(g.lo(), g.hi());
            if x < mid {
                if let Some(left) = g.left() {
                    current = left;
                } else {
                    return current;
                }
            } else if let Some(right) = g.right() {
                current = right;
            } else {
                return current;
            }
        }
    }

    // ── Sum recomputation ────────────────────────────────────────────────

    /// Walks from `start` toward the root, recomputing `sum` at each node.
    pub(crate) fn recompute_sums(&mut self, start: GNodeId) {
        let mut current = Some(start);
        while let Some(id) = current {
            let (left_sum, right_sum) = {
                let g = self.nodes.get(id.index());
                let l = g
                    .left()
                    .map_or_else(V::zero, |l| self.nodes.get(l.index()).sum());
                let r = g
                    .right()
                    .map_or_else(V::zero, |r| self.nodes.get(r.index()).sum());
                (l, r)
            };
            let g = self.nodes.get_mut(id.index());
            g.set_sum(V::add(g.own(), V::add(left_sum, right_sum)));
            current = g.parent();
        }
    }

    /// Recomputes `sum` for each node in `preorder` (processed in reverse,
    /// i.e. leaves-first).
    pub(crate) fn recompute_sums_subtree(&mut self, preorder: &[GNodeId]) {
        for &gid in preorder.iter().rev() {
            let (left_sum, right_sum) = {
                let g = self.nodes.get(gid.index());
                let l = g
                    .left()
                    .map_or_else(V::zero, |l| self.nodes.get(l.index()).sum());
                let r = g
                    .right()
                    .map_or_else(V::zero, |r| self.nodes.get(r.index()).sum());
                (l, r)
            };
            let g = self.nodes.get_mut(gid.index());
            g.set_sum(V::add(g.own(), V::add(left_sum, right_sum)));
        }
    }

    // ── Depth helpers ────────────────────────────────────────────────────

    /// Returns the uniform contour depth of the subtree rooted at `gid`, or
    /// `None` if the leaf G-nodes in the subtree do not all share the same
    /// depth.  `n` is the bit-width of the coordinate domain.
    #[cfg(feature = "dynamic-contour-tracking")]
    #[must_use]
    pub(crate) fn uniform_contour_depth_of(&self, gid: GNodeId, n: u32) -> Option<u32> {
        uniform_contour_depth_of(&self.nodes, gid, n)
    }

    // ── G-node allocation / eviction helpers ─────────────────────────────

    /// Allocates the missing child of a semi-internal node and links it into
    /// the vacant slot.
    ///
    /// This helper intentionally does not update `node_count` or
    /// `terminal_count`; legacy-promote batching accounts for that separately.
    pub(crate) fn allocate_missing_child(&mut self, parent_id: GNodeId) -> GNodeId {
        let (new_lo, new_hi) = self
            .nodes
            .get(parent_id.index())
            .uncovered_range()
            .expect("allocate_missing_child: parent must have uncovered range");
        let new_child = GNode::new_leaf(new_lo, new_hi, V::zero(), Some(parent_id));
        let new_child_id = GNodeId::from_index(self.alloc(new_child));

        let parent = self.nodes.get_mut(parent_id.index());
        if parent.left().is_none() {
            parent.link_left(new_child_id);
        } else {
            debug_assert!(
                parent.right().is_none(),
                "allocate_missing_child: expected empty right slot"
            );
            parent.link_right(new_child_id);
        }

        new_child_id
    }

    /// Allocates two new leaf G-nodes that split `parent_id` at its midpoint,
    /// links them as left and right children of `parent_id`, and updates the
    /// node / terminal counts (`node_count += 2`, `terminal_count += 1`).
    /// Returns `(left_id, right_id)`.
    pub(crate) fn allocate_children(&mut self, parent_id: GNodeId) -> (GNodeId, GNodeId) {
        let (lo, hi) = {
            let g = self.nodes.get(parent_id.index());
            (g.lo(), g.hi())
        };
        let mid = C::midpoint(lo, hi);
        let left_id =
            GNodeId::from_index(self.alloc(GNode::new_leaf(lo, mid, V::zero(), Some(parent_id))));
        let right_id =
            GNodeId::from_index(self.alloc(GNode::new_leaf(mid, hi, V::zero(), Some(parent_id))));
        self.nodes.get_mut(parent_id.index()).link_left(left_id);
        self.nodes.get_mut(parent_id.index()).link_right(right_id);
        self.node_count += 2;
        // The parent was a terminal; it is now internal. Two new terminals are
        // added, the parent terminal is lost: net change = +2 − 1 = +1.
        self.terminal_count += 1;
        (left_id, right_id)
    }

    pub(crate) fn assign_entry(&mut self, gnode_id: GNodeId, entry_id: VNodeId) {
        self.nodes.get_mut(gnode_id.index()).assign_entry(entry_id);
    }

    pub(crate) fn clear_entry(&mut self, gnode_id: GNodeId) {
        self.nodes.get_mut(gnode_id.index()).clear_entry();
    }

    /// Absorbs `child_id`'s accumulated sum into its parent's own weight,
    /// clears the child link from the parent, and recomputes the parent's sum
    /// invariant.  Returns the parent `GNodeId`.
    ///
    /// **Note:** this method does *not* deallocate `child_id` — the caller is
    /// responsible for deallocation after any remaining cross-tree work that
    /// may still read the child slot (e.g. `vtree_remove_leaf`).
    pub(crate) fn merge_into_parent(&mut self, child_id: GNodeId) -> GNodeId {
        let parent_id = self
            .nodes
            .get(child_id.index())
            .parent()
            .expect("merge_into_parent: child must have a parent");

        let child_sum = self.nodes.get(child_id.index()).sum();
        let parent_own_before = self.nodes.get(parent_id.index()).own();
        self.nodes
            .get_mut(parent_id.index())
            .set_own(V::add(parent_own_before, child_sum));
        self.nodes.get_mut(parent_id.index()).detach_child(child_id);

        let new_sum = {
            let p = self.nodes.get(parent_id.index());
            let left_sum = p
                .left()
                .map_or_else(V::zero, |l| self.nodes.get(l.index()).sum());
            let right_sum = p
                .right()
                .map_or_else(V::zero, |r| self.nodes.get(r.index()).sum());
            V::add(p.own(), V::add(left_sum, right_sum))
        };
        self.nodes.get_mut(parent_id.index()).set_sum(new_sum);

        parent_id
    }
}

/// Returns the uniform contour depth of the subtree rooted at `gid`, or
/// `None` if the leaf G-nodes in that subtree do not all share the same depth.
///
/// Kept as a free helper (instead of an inherent method only) so low-level
/// code can evaluate synthetic `Arena<GNode<..>>` fixtures without constructing
/// a full `GNodeTree` wrapper (root/counters are irrelevant for this query).
#[cfg(feature = "dynamic-contour-tracking")]
#[must_use]
pub fn uniform_contour_depth_of<C: Coordinate, V: Accumulator>(
    gnodes: &Arena<GNode<C, V>>,
    gid: GNodeId,
    n: u32,
) -> Option<u32> {
    use super::gnode::GState;

    let g = gnodes.get(gid.index());
    match g.state() {
        GState::Terminal => Some(super::gnode_depth_from_range(g.range(), n)),
        GState::SemiInternal => None,
        GState::Internal => {
            let ld = g
                .left()
                .and_then(|l| uniform_contour_depth_of(gnodes, l, n))?;
            let rd = g
                .right()
                .and_then(|r| uniform_contour_depth_of(gnodes, r, n))?;
            if ld == rd { Some(ld) } else { None }
        }
    }
}
