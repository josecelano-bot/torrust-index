use crate::arena::Arena;
use crate::handle::VNodeId;
use crate::nodes::vnode::{VKind, VNode};
use crate::traits::Accumulator;

pub fn replace_child_in_parent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    parent: VNodeId,
    old_child: VNodeId,
    new_child: VNodeId,
    new_intensity: V,
) {
    let p = vnodes.get_mut(parent.index());
    if let VKind::Structural { children, .. } = p.kind_mut() {
        children.replace_child(old_child, new_child, new_intensity);
    }
}

pub fn add_child_to_structural<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    parent: VNodeId,
    child: VNodeId,
    child_intensity: V,
) {
    let p = vnodes.get_mut(parent.index());
    if let VKind::Structural { children, .. } = p.kind_mut() {
        children.add_child(child, child_intensity);
    }
}

pub fn set_entry_flags<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    entry_id: VNodeId,
    is_exposed_value: bool,
    is_evictable_value: bool,
) {
    if let VKind::Entry {
        is_exposed,
        is_evictable,
        ..
    } = vnodes.get_mut(entry_id.index()).kind_mut()
    {
        *is_exposed = is_exposed_value;
        *is_evictable = is_evictable_value;
    }
}

pub fn remove_child_from_structural<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    parent: VNodeId,
    child: VNodeId,
) {
    let p = vnodes.get_mut(parent.index());
    if let VKind::Structural { children, .. } = p.kind_mut() {
        children.remove_child(child);
    }
}

pub fn set_has_evictable<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    id: VNodeId,
    flag: bool,
) {
    let node = vnodes.get_mut(id.index());
    if let VKind::Structural { has_evictable, .. } = node.kind_mut() {
        *has_evictable = flag;
    }
}
