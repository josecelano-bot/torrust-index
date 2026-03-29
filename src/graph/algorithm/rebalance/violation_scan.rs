use crate::arena::Arena;
use crate::handle::VNodeId;
use crate::nodes::vnode::VNode;
use crate::traits::Accumulator;

use super::is_violated;

fn v_depth_local<V: Accumulator>(vnodes: &Arena<VNode<V>>, id: VNodeId) -> u32 {
    let node = vnodes.get(id.index());
    node.parent().map_or(0, |p| v_depth_local(vnodes, p) + 1)
}

#[must_use]
pub fn find_violated_nodes<V: Accumulator>(vnodes: &Arena<VNode<V>>) -> Vec<VNodeId> {
    let mut violated: Vec<(VNodeId, u32)> = Vec::new();
    for (idx, _) in vnodes.iter_occupied() {
        let id = VNodeId::from_index(idx);
        if is_violated(vnodes, id) {
            violated.push((id, v_depth_local(vnodes, id)));
        }
    }

    violated.sort_by_key(|b| std::cmp::Reverse(b.1));
    violated.into_iter().map(|(id, _)| id).collect()
}
