use crate::graph::GvGraph;
use crate::graph::algorithm::rebalance::{Nd, contract};
use crate::graph::algorithm::violation_push::ViolationQueue;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::VNode;
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::gtree::GTree;
use crate::tree::vtree::VTree;

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvGraph<C, V, N> {
    pub(super) fn split_candidate_entry(&self, g_id: GNodeId) -> Option<VNodeId> {
        let g = self.core.gtree.nodes.get(g_id.index());
        if g.left().is_some() || g.right().is_some() {
            return None;
        }

        let mid = C::midpoint(g.lo(), g.hi());
        if mid.partial_cmp(&g.lo()) != Some(std::cmp::Ordering::Greater) {
            return None;
        }

        if g.sum().partial_cmp(&self.config.split_threshold) != Some(std::cmp::Ordering::Greater) {
            return None;
        }

        g.entry()
    }

    pub(super) fn preprocess_split_parent(&mut self, entry_id: VNodeId) {
        let p_id = self
            .core
            .vtree
            .nodes
            .get(entry_id.index())
            .parent()
            .unwrap();
        if !self
            .core
            .vtree
            .nodes
            .get(p_id.index())
            .is_structural_triple()
        {
            return;
        }

        let _span = tracing::debug_span!(
            "split_preprocess",
            p = %Nd(&self.core.vtree.nodes, p_id),
        )
        .entered();
        let merged = contract(&mut self.core.vtree, p_id);
        let mut queue = ViolationQueue::new(&mut self.core.vtree.violations);
        queue.push_side_effect(&self.core.vtree.nodes, p_id);
        queue.push_side_effect(&self.core.vtree.nodes, merged);
        queue.push_promoted(&self.core.vtree.nodes, p_id);
    }

    pub(super) fn allocate_split_children(&mut self, g_id: GNodeId) -> SplitChildren {
        let (left_id, right_id) = self.core.gtree.nodes.allocate_children(g_id);
        let left_entry_id = alloc_v_entry(&mut self.core.vtree, &mut self.core.gtree, left_id);
        let right_entry_id = alloc_v_entry(&mut self.core.vtree, &mut self.core.gtree, right_id);

        SplitChildren {
            left_id,
            left_entry_id,
            right_entry_id,
        }
    }

    pub(super) fn debug_assert_split_mirror_consistency(&self, label: &str) {
        if tracing::enabled!(tracing::Level::DEBUG) {
            self.debug_assert_plateau_mirror_consistency(label);
        }
    }
}

pub(super) struct SplitChildren {
    pub(super) left_id: GNodeId,
    pub(super) left_entry_id: VNodeId,
    pub(super) right_entry_id: VNodeId,
}

pub(super) fn alloc_v_entry<C: Coordinate, V: Accumulator, const N: u32>(
    vtree: &mut VTree<V>,
    gtree: &mut GTree<C, V, N>,
    gnode: GNodeId,
) -> VNodeId {
    let e = VNode::new_entry(V::zero(), None, gnode, true, true);
    let e_id = VNodeId::from_index(vtree.nodes.alloc(e).0);
    gtree.nodes.assign_entry(gnode, e_id);
    e_id
}
