use crate::arena::Arena;
use crate::graph::GvGraph;
use crate::graph::algorithm::rebalance::{Nd, contract};
use crate::graph::algorithm::violation_push::{
    ViolationQueue,
};
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::gnode::GNode;
use crate::nodes::vnode::{Children, VNode};
use crate::traits::{Accumulator, Coordinate, Inspectable};

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvGraph<C, V, N> {
    pub(super) fn split_candidate_entry(&self, g_id: GNodeId) -> Option<VNodeId> {
        let g = self.gtree.nodes.get(g_id.index());
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
        let p_id = self.vtree.nodes.get(entry_id.index()).parent().unwrap();
        if !self.vtree.nodes.get(p_id.index()).is_structural_triple() {
            return;
        }

        let _span = tracing::debug_span!(
            "split_preprocess",
            p = %Nd(&self.vtree.nodes, p_id),
        )
        .entered();
        let merged = contract(&mut self.vtree, p_id);
        let mut queue = ViolationQueue::new(&mut self.vtree.violations);
        queue.push_side_effect(&self.vtree.nodes, p_id);
        queue.push_side_effect(&self.vtree.nodes, merged);
        queue.push_promoted(&self.vtree.nodes, p_id);
    }

    pub(super) fn allocate_split_children(&mut self, g_id: GNodeId) -> SplitChildren {
        let (left_id, right_id) = self.gtree.allocate_children(g_id);
        let left_entry_id = alloc_v_entry(&mut self.vtree.nodes, &mut self.gtree.nodes, left_id);
        let right_entry_id = alloc_v_entry(&mut self.vtree.nodes, &mut self.gtree.nodes, right_id);

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

pub(super) fn alloc_v_entry<C: Coordinate, V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    gnodes: &mut Arena<GNode<C, V>>,
    gnode: GNodeId,
) -> VNodeId {
    let e = VNode::new_entry(V::zero(), None, gnode, true, true);
    let e_id = VNodeId::from_index(vnodes.alloc(e));
    gnodes.get_mut(gnode.index()).assign_entry(e_id);
    e_id
}

pub(super) fn alloc_v_structural_2<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    a: VNodeId,
    b: VNodeId,
) -> VNodeId {
    let a_int = vnodes.get(a.index()).intensity();
    let b_int = vnodes.get(b.index()).intensity();
    let s = VNode::new_structural(
        V::add(a_int, b_int),
        None,
        Children::new_2((a, a_int), (b, b_int)),
        true,
    );
    let s_id = VNodeId::from_index(vnodes.alloc(s));
    vnodes.get_mut(a.index()).set_parent(s_id);
    vnodes.get_mut(b.index()).set_parent(s_id);
    s_id
}
