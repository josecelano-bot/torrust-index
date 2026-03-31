use crate::graph::algorithm::rebalance;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::VNode;
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::gtree::GTree;
use crate::tree::vtree::VTree;

/// The dual-tree core of a `GvGraph`.
///
/// Owns the G-tree (geometric partition skeleton) and the V-tree
/// (intensity-aggregation overlay).  Operations that span both trees belong
/// here; operations that also need `config` or the plateau `tracker` live on
/// `GvGraph` instead.
pub struct GvCore<C: Coordinate, V: Accumulator, const N: u32> {
    pub(crate) gtree: GTree<C, V, N>,
    pub(crate) vtree: VTree<V>,
}

impl<C: Coordinate, V: Accumulator, const N: u32> Clone for GvCore<C, V, N> {
    fn clone(&self) -> Self {
        Self {
            gtree: self.gtree.clone(),
            vtree: self.vtree.clone(),
        }
    }
}

impl<C: Coordinate, V: Accumulator, const N: u32> GvCore<C, V, N> {
    pub(crate) fn alloc_v_entry(&mut self, gnode: GNodeId) -> VNodeId {
        let e = VNode::new_entry(V::zero(), None, gnode, true, true);
        let e_id = VNodeId::from_index(self.vtree.nodes.alloc(e).0);
        self.gtree.nodes.assign_entry(gnode, e_id);
        e_id
    }
}

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvCore<C, V, N> {
    pub(crate) fn rebalance(&mut self) -> Vec<GNodeId> {
        let depth_evict = self.gtree.live_depth_evict;
        rebalance::rebalance(&mut self.vtree, &mut self.gtree, depth_evict)
    }
}
