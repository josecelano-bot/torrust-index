use crate::arena::Arena;
use crate::handle::VNodeId;
use crate::nodes::vnode::{Children, VKind, VNode};
use crate::traits::Accumulator;
use std::ops::{Deref, DerefMut};

/// Structural V-node owner.
#[derive(Debug, Clone)]
pub struct VNodeTree<V: Accumulator> {
    nodes: Arena<VNode<V>>,
}

impl<V: Accumulator> VNodeTree<V> {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { nodes: Arena::new() }
    }
}

impl<V: Accumulator> Default for VNodeTree<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Accumulator> From<Arena<VNode<V>>> for VNodeTree<V> {
    fn from(nodes: Arena<VNode<V>>) -> Self {
        Self { nodes }
    }
}

impl<V: Accumulator> Deref for VNodeTree<V> {
    type Target = Arena<VNode<V>>;

    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl<V: Accumulator> DerefMut for VNodeTree<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

impl<V: Accumulator> VNodeTree<V> {
    #[must_use]
    pub(crate) fn depth(&self, id: VNodeId) -> u32 {
        let node = self.get(id.index());
        node.parent().map_or(0, |p| self.depth(p) + 1)
    }

    pub(crate) fn compute_has_evictable(&self, children: &Children<V>) -> bool {
        for i in 0..children.len() {
            let (child_id, _) = children.get(i);
            let child = self.get(child_id.index());
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

    #[must_use]
    pub(crate) fn node_has_evictable(&self, id: VNodeId) -> bool {
        match &self.get(id.index()).kind() {
            VKind::Entry { is_evictable, .. } => *is_evictable,
            VKind::Structural { has_evictable, .. } => *has_evictable,
        }
    }

    #[must_use]
    pub(crate) fn sibling_of(&self, parent: VNodeId, child: VNodeId) -> (VNodeId, V) {
        let p_node = self.get(parent.index());
        match &p_node.kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(
                    children.len(),
                    2,
                    "sibling_of: parent must be a 2-child structural node"
                );

                let (id0, int0) = children.get(0);
                let (id1, int1) = children.get(1);

                if id0 == child {
                    (id1, int1)
                } else if id1 == child {
                    (id0, int0)
                } else {
                    panic!(
                        "sibling_of: child {} not found in parent {}",
                        child.index(),
                        parent.index()
                    );
                }
            }
            VKind::Entry { .. } => panic!("sibling_of: parent must be structural"),
        }
    }
}
