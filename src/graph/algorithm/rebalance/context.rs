use crate::tree::handle::VNodeId;

pub use super::super::fmt::{Ctx, Nd};

#[derive(Debug, Clone, Copy)]
pub struct EscalationContext {
    pub parent_id: VNodeId,
    pub grandparent_id: VNodeId,
    pub heaviest_id: VNodeId,
    pub merged_id: Option<VNodeId>,
    pub grandparent_merged_id: Option<VNodeId>,
    pub heaviest_is_direct_child: bool,
}

impl EscalationContext {
    #[must_use]
    pub const fn new(
        parent_id: VNodeId,
        grandparent_id: VNodeId,
        heaviest_id: VNodeId,
        heaviest_is_direct_child: bool,
    ) -> Self {
        Self {
            parent_id,
            grandparent_id,
            heaviest_id,
            merged_id: None,
            grandparent_merged_id: None,
            heaviest_is_direct_child,
        }
    }
}
