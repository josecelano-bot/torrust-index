use crate::handle::VNodeId;
use crate::traits::Accumulator;
use crate::tree::vtree::VNodeTree;

use super::is_violated;

#[must_use]
pub fn find_violated_nodes<V: Accumulator>(vnodes: &VNodeTree<V>) -> Vec<VNodeId> {
    let mut violated: Vec<(VNodeId, u32)> = Vec::new();
    for (idx, _) in vnodes.iter_occupied() {
        let id = VNodeId::from_index(idx);
        if is_violated(vnodes, id) {
            violated.push((id, vnodes.depth(id)));
        }
    }

    violated.sort_by_key(|b| std::cmp::Reverse(b.1));
    violated.into_iter().map(|(id, _)| id).collect()
}
