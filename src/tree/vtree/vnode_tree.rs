use crate::arena::Arena;
use crate::nodes::vnode::VNode;
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
