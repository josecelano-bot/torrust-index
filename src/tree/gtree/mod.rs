use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::GNode;
use crate::spatial::range::CoordinateRange;
use crate::traits::{Accumulator, Coordinate};

pub(crate) mod gnode_tree;
pub(crate) use gnode_tree::GNodeTree;

// ── GTree ────────────────────────────────────────────────────────────────────

/// The G-tree: a binary spatial-partition tree whose leaves are the observable
/// coordinate ranges.  Wraps [`GNodeTree`] and adds policy parameters that
/// constrain which trees are valid in the domain context.
#[derive(Debug, Clone)]
pub struct GTree<C: Coordinate, V: Accumulator, const N: u32> {
    /// Structural node container (backing store, root, counters).
    pub(crate) nodes: GNodeTree<C, V>,
    /// Maximum depth at which live V-entries can exist before eviction.
    pub(crate) live_depth_evict: u32,
    /// Maximum depth at which new V-entries are created.
    pub(crate) live_depth_create: u32,
    /// `depth_evict - depth_create`.
    pub(crate) depth_buffer: u32,
    /// Maximum nodes in the `[depth_create, depth_evict]` band.
    pub(crate) headroom: usize,
    /// Soft node-count limit that triggers eviction (`budget - headroom`).
    pub(crate) soft_limit: Option<usize>,
}


impl<C: Coordinate, V: Accumulator, const N: u32> GTree<C, V, N> {
    // ── Depth helpers (policy-bound) ─────────────────────────────────────

    /// Returns the depth of a G-node whose interval is `[lo, hi)` in an
    /// `N`-bit domain.
    #[must_use]
    #[inline]
    pub(crate) fn depth_of_interval(lo: C, hi: C) -> u32 {
        gnode_depth_from_interval(lo, hi, N)
    }

    /// Returns the uniform contour depth of the subtree rooted at `gid`, or
    /// `None` if the leaf G-nodes in the subtree do not all share the same depth.
    #[cfg(feature = "dynamic-contour-tracking")]
    #[must_use]
    pub(crate) fn uniform_contour_depth(&self, gid: GNodeId) -> Option<u32> {
        self.nodes.uniform_contour_depth_of(gid, N)
    }

    // ── G-node allocation helper (counter-exempt) ─────────────────────────

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
        let new_child_id = GNodeId::from_index(self.nodes.alloc(new_child));

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
}

// ── Free functions ────────────────────────────────────────────────────────────

#[must_use]
#[inline]
pub fn gnode_depth_from_interval<C: Coordinate>(lo: C, hi: C, n: u32) -> u32 {
    gnode_depth_from_range(CoordinateRange::new(lo, hi), n)
}

#[must_use]
#[inline]
pub fn gnode_depth_from_range<C: Coordinate>(range: CoordinateRange<C>, n: u32) -> u32 {
    let width_f64 = C::width(range.lo, range.hi).to_f64();
    debug_assert!(
        width_f64 > 0.0,
        "gnode_depth_from_interval: zero-width interval"
    );

    #[allow(clippy::cast_possible_truncation)]
    let log2_width = width_f64.log2() as i32;
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    let depth = (n as i32 - log2_width) as u32;
    depth
}

/// Returns the uniform contour depth of the subtree rooted at `gid`, or `None`
/// if the leaf G-nodes do not all share the same depth.
#[cfg(feature = "dynamic-contour-tracking")]
#[must_use]
pub fn uniform_contour_depth_of<C: Coordinate, V: Accumulator>(
    gnodes: &Arena<GNode<C, V>>,
    gid: GNodeId,
    n: u32,
) -> Option<u32> {
    use crate::nodes::gnode::GState;
    let g = gnodes.get(gid.index());
    match g.state() {
        GState::Terminal => Some(gnode_depth_from_range(g.range(), n)),
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
