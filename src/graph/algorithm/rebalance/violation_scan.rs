use crate::handle::VNodeId;
use crate::traits::Accumulator;
use crate::tree::vtree::VNodeTree;

#[must_use]
pub fn find_violated_nodes<V: Accumulator>(vnodes: &VNodeTree<V>) -> Vec<VNodeId> {
    vnodes.find_violated_nodes()
}
