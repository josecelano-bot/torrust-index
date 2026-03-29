use crate::arena::Arena;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::gnode::GNode;
use crate::nodes::vnode::{VKind, VNode};
use crate::traits::{Accumulator, Coordinate};

use super::super::promote::{legacy_promote, skip_promote, standard_promote};
use super::super::violation_push::{
    ViolationQueue,
};
use super::{contract, is_violated, Ctx, EscalationContext, Nd, VTreeMutContext};

fn structural_child_count<V: Accumulator>(vnodes: &Arena<VNode<V>>, id: VNodeId) -> usize {
    vnodes.get(id.index()).child_count()
}

fn any_child_violated<V: Accumulator>(vnodes: &Arena<VNode<V>>, node: VNodeId) -> bool {
    match &vnodes.get(node.index()).kind() {
        VKind::Structural { children, .. } => {
            for i in 0..children.len() {
                let (child_id, _) = children.get(i);
                if is_violated(vnodes, child_id) {
                    return true;
                }
            }
            false
        }
        VKind::Entry { .. } => false,
    }
}

fn v_depth_local<V: Accumulator>(vnodes: &Arena<VNode<V>>, id: VNodeId) -> u32 {
    let node = vnodes.get(id.index());
    node.parent().map_or(0, |p| v_depth_local(vnodes, p) + 1)
}

/// Contracts the 3-child parent `p` after a promote, propagates violations,
/// and returns `Some(merged)` if the violation persists (Phase 3 needed),
/// or `None` if the contraction resolved it.
fn escalate_contract_parent<V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    ctx: &mut EscalationContext,
) -> Option<VNodeId> {
    let (vnodes, violations) = (&mut *tree.vnodes, &mut *tree.violations);
    let merged = contract(vnodes, ctx.parent_id);
    let mut queue = ViolationQueue::new(violations);
    queue.push_side_effect(vnodes, ctx.parent_id);
    queue.push_side_effect(vnodes, merged);
    queue.push_contraction_child(vnodes, ctx.parent_id, ctx.heaviest_id);
    ctx.merged_id = Some(merged);

    let needs_skip = if ctx.heaviest_is_direct_child {
        is_violated(vnodes, ctx.heaviest_id)
    } else {
        is_violated(vnodes, merged)
    };
    if needs_skip {
        Some(merged)
    } else {
        tracing::debug!("resolved by contraction");
        None
    }
}

/// Optionally contracts the grandparent `g` (when it is a 3-node), propagates
/// violations, and returns `(g_merged, resolved)`.
///
/// `resolved = true` means the violation was resolved and the caller should
/// return immediately.
fn escalate_try_contract_grandparent<V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    ctx: &mut EscalationContext,
) -> bool {
    let (vnodes, violations) = (&mut *tree.vnodes, &mut *tree.violations);
    if structural_child_count(vnodes, ctx.grandparent_id) == 3 {
        let g_merged = contract(vnodes, ctx.grandparent_id);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, ctx.grandparent_id);
        queue.push_side_effect(vnodes, g_merged);
        queue.push_promoted(vnodes, ctx.grandparent_id);
        ctx.grandparent_merged_id = Some(g_merged);

        let merged = ctx
            .merged_id
            .expect("escalate_try_contract_grandparent: parent contraction must run first");
        let resolved = !is_violated(vnodes, ctx.heaviest_id)
            && (ctx.heaviest_is_direct_child || !is_violated(vnodes, merged));
        if resolved {
            tracing::debug!("resolved by g-contraction");
            return true;
        }
    }

    false
}

/// Skip-promote fallback (Phase 4): moves the violation upward when neither
/// parent nor grandparent contraction resolved it.
fn escalate_skip_promote<V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    ctx: &EscalationContext,
) {
    let (vnodes, violations) = (&mut *tree.vnodes, &mut *tree.violations);
    if let Some(g_id) = vnodes.get(ctx.parent_id.index()).parent() {
        skip_promote(vnodes, ctx.heaviest_id);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, g_id);
        queue.push_promoted(vnodes, g_id);

        if let Some(gm) = ctx.grandparent_merged_id {
            queue.push_source_10(vnodes, gm);
        }
    }
}

fn escalate_after_promote<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    p: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    // Phase 1: Identify heaviest child; early-return if no violation.
    let p_node = vnodes.get(p.index());
    if !p_node.is_structural_triple() {
        return;
    }
    let heaviest = match p_node.kind() {
        VKind::Structural { children, .. } => children.get(children.heaviest_child_index()).0,
        VKind::Entry { .. } => return,
    };

    let h_direct = is_violated(vnodes, heaviest);
    let h_indirect = !h_direct && any_child_violated(vnodes, heaviest);
    if !h_direct && !h_indirect {
        return;
    }

    let Some(g) = vnodes.get(p.index()).parent() else {
        return;
    };
    let _span = tracing::debug_span!(
        "escalate",
        h = %Nd(vnodes, heaviest),
        reason = if h_direct { "direct" } else { "indirect" },
    )
    .entered();

    let mut tree = VTreeMutContext { vnodes, violations };
    let mut ctx = EscalationContext::new(p, g, heaviest, h_direct);

    // Phase 2: Contract 3-child parent `p` and propagate violations.
    let Some(_merged) = escalate_contract_parent(&mut tree, &mut ctx) else {
        return;
    };

    // Phase 3: Optionally contract grandparent `g` if it has 3 children.
    if escalate_try_contract_grandparent(&mut tree, &mut ctx) {
        return;
    }

    // Phase 4: Skip-promote fallback.
    escalate_skip_promote(&mut tree, &ctx);
}

/// Phase 1 of `resolve`: if the parent of `c` is a 3-child node, contract it
/// and propagate side-effect violations. Returns `true` if the contraction
/// resolved `c`'s violation (caller should return `None` immediately).
fn resolve_try_contract_parent<V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    p: VNodeId,
    c: VNodeId,
) -> bool {
    let (vnodes, violations) = (&mut *tree.vnodes, &mut *tree.violations);
    if structural_child_count(vnodes, p) == 3 {
        tracing::debug!(p = %Nd(vnodes, p), "phase 1: contracting 3-node parent");
        let merged = contract(vnodes, p);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, p);
        queue.push_side_effect(vnodes, merged);
        queue.push_contraction_child(vnodes, p, c);
        if !is_violated(vnodes, c) {
            tracing::debug!("phase 1: resolved by contraction");
            return true;
        }
    }
    false
}

/// Path B of `resolve`: handles skip / legacy promote.
///
/// Attempts a grandparent contraction first, then either legacy-promotes (when
/// `c` is a semi-internal entry at or above `depth_evict`) or skip-promotes.
/// Returns `Some(new_g)` only when a legacy promote created a new G-node.
fn resolve_path_b<C: Coordinate, V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    gnodes: &mut Arena<GNode<C, V>>,
    c: VNodeId,
    p: VNodeId,
    g: VNodeId,
    depth_evict: u32,
) -> Option<GNodeId> {
    let (vnodes, violations) = (&mut *tree.vnodes, &mut *tree.violations);
    // Optional grandparent contraction before the promote attempt.
    let g_merged = if structural_child_count(vnodes, g) == 3 {
        let merged = contract(vnodes, g);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, g);
        queue.push_side_effect(vnodes, merged);
        queue.push_promoted(vnodes, g);
        if !is_violated(vnodes, c) {
            tracing::debug!("phase 2: resolved by g-contraction");
            return None;
        }
        Some(merged)
    } else {
        None
    };

    let Some(g_id) = vnodes.get(p.index()).parent() else {
        tracing::warn!(
            node = %Ctx(vnodes, c),
            "skip-promote path: no grandparent after g-contraction — resolve incomplete",
        );
        return None;
    };

    let is_semi = matches!(
        &vnodes.get(c.index()).kind(),
        VKind::Entry { gnode, .. }
            if gnodes.get(gnode.index()).is_semi_internal()
    );

    let result = if is_semi && v_depth_local(vnodes, c) <= depth_evict {
        tracing::debug!("phase 2: legacy promote (semi-internal entry)");
        let new_g = legacy_promote(vnodes, gnodes, c);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, p);
        Some(new_g)
    } else {
        skip_promote(vnodes, c);
        None
    };

    let mut queue = ViolationQueue::new(violations);
    queue.push_side_effect(vnodes, g_id);
    queue.push_promoted(vnodes, g_id);
    if let Some(merged) = g_merged {
        queue.push_source_10(vnodes, merged);
    }

    result
}

/// Attempt to resolve a single violation at V-node `c`.
///
/// The function dispatches between two paths based on the shape of `c`:
///
/// - Path A (standard promote): `c` is a structural node with exactly two
///   children. The violation is resolved by calling `standard_promote` and
///   propagating any side-effects upward. This is the common, cheap case.
///
/// - Path B (skip/legacy promote): `c` is any other kind (entry, or
///   structural with != 2 children). An optional grandparent contraction is
///   attempted first; then, if `c` is a semi-internal entry at or above
///   `depth_evict`, a `legacy_promote` upgrades it to a full G-node (returning
///   the new `GNodeId`); otherwise a `skip_promote` moves the violation up.
///
/// In both paths a preliminary contraction of the parent is attempted when the
/// parent has 3 structural children, which may resolve the violation outright
/// before the main dispatch.
///
/// Returns `Some(new_g)` only when a legacy promote created a new G-node.
pub fn resolve<C: Coordinate, V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    gnodes: &mut Arena<GNode<C, V>>,
    c: VNodeId,
    depth_evict: u32,
) -> Option<GNodeId> {
    let _span = tracing::debug_span!("resolve", node = c.index()).entered();
    tracing::debug!(ctx = %Ctx(tree.vnodes, c), "begin");

    let Some(p) = tree.vnodes.get(c.index()).parent() else {
        tracing::trace!("no parent — nothing to resolve");
        return None;
    };

    // Phase 1: optional parent contraction.
    if resolve_try_contract_parent(tree, p, c) {
        return None;
    }

    // Path A: standard promote.
    if tree.vnodes.get(c.index()).is_structural_pair() {
        tracing::debug!("phase 2: standard promote");
        standard_promote(tree.vnodes, c);
        let mut queue = ViolationQueue::new(tree.violations);
        queue.push_side_effect(tree.vnodes, p);
        queue.push_promoted(tree.vnodes, p);
        escalate_after_promote(tree.vnodes, p, tree.violations);
        return None;
    }

    // Path B: skip / legacy promote.
    tracing::debug!("phase 2: skip promote path");
    let Some(g) = tree.vnodes.get(p.index()).parent() else {
        tracing::trace!("no grandparent — cannot skip-promote");
        return None;
    };

    let result = resolve_path_b(tree, gnodes, c, p, g, depth_evict);

    if tree.vnodes.is_occupied(c.index()) && is_violated(tree.vnodes, c) {
        tracing::warn!(
            node = %Ctx(tree.vnodes, c),
            "resolve() returning with node STILL violated",
        );
    }

    result
}
