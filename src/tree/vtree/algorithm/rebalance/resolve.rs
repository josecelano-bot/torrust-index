use crate::graph::algorithm::fmt::Nd;
use crate::graph::algorithm::violation_push::ViolationQueue;
use crate::traits::Accumulator;
use crate::tree::handle::VNodeId;
use crate::tree::vtree::VTree;
use crate::tree::vtree::vnode::VKind;

use super::context::EscalationContext;

// ── VTree methods ─────────────────────────────────────────────────────────────

impl<V: Accumulator> VTree<V> {
    /// Contracts the 3-child parent `p` after a promote, propagates violations,
    /// and returns `Some(merged)` if the violation persists (Phase 3 needed),
    /// or `None` if the contraction resolved it.
    fn escalate_contract_parent(&mut self, ctx: &mut EscalationContext) -> Option<VNodeId> {
        let merged = self.contract(ctx.parent_id);
        {
            let (vnodes, violations) = (&self.nodes, &mut self.violations);
            let mut queue = ViolationQueue::new(violations);
            queue.push_side_effect(vnodes, ctx.parent_id);
            queue.push_side_effect(vnodes, merged);
            queue.push_contraction_child(vnodes, ctx.parent_id, ctx.heaviest_id);
        }
        ctx.merged_id = Some(merged);

        let needs_skip = if ctx.heaviest_is_direct_child {
            self.nodes.is_violated(ctx.heaviest_id)
        } else {
            self.nodes.is_violated(merged)
        };
        if needs_skip {
            Some(merged)
        } else {
            tracing::debug!("resolved by contraction");
            None
        }
    }

    /// Optionally contracts the grandparent `g` (when it is a 3-node), propagates
    /// violations, and returns `resolved = true` when the violation is resolved
    /// and the caller should return immediately.
    fn escalate_try_contract_grandparent(&mut self, ctx: &mut EscalationContext) -> bool {
        if self.nodes.structural_child_count(ctx.grandparent_id) == 3 {
            let g_merged = self.contract(ctx.grandparent_id);
            {
                let (vnodes, violations) = (&self.nodes, &mut self.violations);
                let mut queue = ViolationQueue::new(violations);
                queue.push_side_effect(vnodes, ctx.grandparent_id);
                queue.push_side_effect(vnodes, g_merged);
                queue.push_promoted(vnodes, ctx.grandparent_id);
            }
            ctx.grandparent_merged_id = Some(g_merged);

            let merged = ctx
                .merged_id
                .expect("escalate_try_contract_grandparent: parent contraction must run first");
            let resolved = !self.nodes.is_violated(ctx.heaviest_id)
                && (ctx.heaviest_is_direct_child || !self.nodes.is_violated(merged));
            if resolved {
                tracing::debug!("resolved by g-contraction");
                return true;
            }
        }

        false
    }

    /// Skip-promote fallback (Phase 4): moves the violation upward when neither
    /// parent nor grandparent contraction resolved it.
    fn escalate_skip_promote(&mut self, ctx: &EscalationContext) {
        let g_id_opt = self.nodes.get(ctx.parent_id.index()).parent();
        if let Some(g_id) = g_id_opt {
            self.skip_promote(ctx.heaviest_id);
            let (vnodes, violations) = (&self.nodes, &mut self.violations);
            let mut queue = ViolationQueue::new(violations);
            queue.push_side_effect(vnodes, g_id);
            queue.push_promoted(vnodes, g_id);

            if let Some(gm) = ctx.grandparent_merged_id {
                queue.push_source_10(vnodes, gm);
            }
        }
    }

    pub fn escalate_after_promote(&mut self, p: VNodeId) {
        // Phase 1: Identify heaviest child; early-return if no violation.
        // Scope the shared borrow so it is dropped before the mutable helper calls.
        let (heaviest, h_direct, g) = {
            let vnodes = &self.nodes;
            let p_node = vnodes.get(p.index());
            if !p_node.is_structural_triple() {
                return;
            }
            let heaviest = match p_node.kind() {
                VKind::Structural { children, .. } => {
                    children.get(children.heaviest_child_index()).0
                }
                VKind::Entry { .. } => return,
            };
            let h_direct = vnodes.is_violated(heaviest);
            let h_indirect = !h_direct && vnodes.any_child_violated(heaviest);
            if !h_direct && !h_indirect {
                return;
            }
            let Some(g) = vnodes.get(p.index()).parent() else {
                return;
            };
            (heaviest, h_direct, g)
        }; // shared borrow of self.nodes dropped here

        let _span = tracing::debug_span!(
            "escalate",
            h = %Nd(&self.nodes, heaviest),
            reason = if h_direct { "direct" } else { "indirect" },
        )
        .entered();

        let mut ctx = EscalationContext::new(p, g, heaviest, h_direct);

        // Phase 2: Contract 3-child parent `p` and propagate violations.
        let Some(_merged) = self.escalate_contract_parent(&mut ctx) else {
            return;
        };

        // Phase 3: Optionally contract grandparent `g` if it has 3 children.
        if self.escalate_try_contract_grandparent(&mut ctx) {
            return;
        }

        // Phase 4: Skip-promote fallback.
        self.escalate_skip_promote(&ctx);
    }

    /// Phase 1 of `resolve`: if the parent of `c` is a 3-child node, contract it
    /// and propagate side-effect violations. Returns `true` if the contraction
    /// resolved `c`'s violation (caller should return `None` immediately).
    pub fn resolve_try_contract_parent(&mut self, p: VNodeId, c: VNodeId) -> bool {
        if self.nodes.structural_child_count(p) == 3 {
            tracing::debug!(p = %Nd(&self.nodes, p), "phase 1: contracting 3-node parent");
            let merged = self.contract(p);
            {
                let (vnodes, violations) = (&self.nodes, &mut self.violations);
                let mut queue = ViolationQueue::new(violations);
                queue.push_side_effect(vnodes, p);
                queue.push_side_effect(vnodes, merged);
                queue.push_contraction_child(vnodes, p, c);
            }
            if !self.nodes.is_violated(c) {
                tracing::debug!("phase 1: resolved by contraction");
                return true;
            }
        }
        false
    }
}
