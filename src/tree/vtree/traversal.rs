use crate::arena::Arena;
use crate::handle::VNodeId;
use crate::nodes::vnode::{Children, VKind, VNode};
use crate::traits::Accumulator;

#[must_use]
pub(super) fn v_depth<V: Accumulator>(vnodes: &Arena<VNode<V>>, id: VNodeId) -> u32 {
    let node = vnodes.get(id.index());
    node.parent().map_or(0, |p| v_depth(vnodes, p) + 1)
}

pub(super) fn compute_has_evictable<V: Accumulator>(
    vnodes: &Arena<VNode<V>>,
    children: &Children<V>,
) -> bool {
    for i in 0..children.len() {
        let (child_id, _) = children.get(i);
        let child = vnodes.get(child_id.index());
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
