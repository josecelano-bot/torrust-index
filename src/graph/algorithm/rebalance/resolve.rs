use crate::arena::Arena;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::gnode::GNode;
use crate::nodes::vnode::{VKind, VNode};
use crate::traits::{Accumulator, Coordinate};
use crate::tree::vtree::v_depth;

use super::super::promote::{legacy_promote, skip_promote, standard_promote};
use super::super::violation_push::{
    push_contraction_child_violations, push_promoted_violations, push_side_effect_violations,
    push_source_10_violations,
};
use super::{Ctx, Nd, contract, is_violated};

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

/// Contracts the 3-child parent `p` after a promote, propagates violations,
/// and returns `Some(merged)` if the violation persists (Phase 3 needed),
/// or `None` if the contraction resolved it.
fn escalate_contract_parent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    p: VNodeId,
    heaviest: VNodeId,
    h_direct: bool,
    violations: &mut Vec<VNodeId>,
) -> Option<VNodeId> {
    let merged = contract(vnodes, p);
    push_side_effect_violations(vnodes, p, violations);
    push_side_effect_violations(vnodes, merged, violations);
    push_contraction_child_violations(vnodes, p, heaviest, violations);

    let needs_skip = if h_direct {
        is_violated(vnodes, heaviest)
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
    vnodes: &mut Arena<VNode<V>>,
    g: VNodeId,
    heaviest: VNodeId,
    merged: VNodeId,
    h_direct: bool,
    violations: &mut Vec<VNodeId>,
) -> (Option<VNodeId>, bool) {
    if structural_child_count(vnodes, g) == 3 {
        let g_merged = contract(vnodes, g);
        push_side_effect_violations(vnodes, g, violations);
        push_side_effect_violations(vnodes, g_merged, violations);
        push_promoted_violations(vnodes, g, violations);
        let resolved = !is_violated(vnodes, heaviest) && (h_direct || !is_violated(vnodes, merged));
        if resolved {
            tracing::debug!("resolved by g-contraction");
            return (Some(g_merged), true);
        }
        (Some(g_merged), false)
    } else {
        (None, false)
    }
}

/// Skip-promote fallback (Phase 4): moves the violation upward when neither
/// parent nor grandparent contraction resolved it.
fn escalate_skip_promote<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    p: VNodeId,
    heaviest: VNodeId,
    g_merged: Option<VNodeId>,
    violations: &mut Vec<VNodeId>,
) {
    if let Some(g_id) = vnodes.get(p.index()).parent() {
        skip_promote(vnodes, heaviest);
        push_side_effect_violations(vnodes, g_id, violations);
        push_promoted_violations(vnodes, g_id, violations);

        if let Some(gm) = g_merged {
            push_source_10_violations(vnodes, gm, violations);
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

    // Phase 2: Contract 3-child parent `p` and propagate violations.
    let Some(merged) = escalate_contract_parent(vnodes, p, heaviest, h_direct, violations) else {
        return;
    };

    // Phase 3: Optionally contract grandparent `g` if it has 3 children.
    let (g_merged, resolved) =
        escalate_try_contract_grandparent(vnodes, g, heaviest, merged, h_direct, violations);
    if resolved {
        return;
    }

    // Phase 4: Skip-promote fallback.
    escalate_skip_promote(vnodes, p, heaviest, g_merged, violations);
}

/// Phase 1 of `resolve`: if the parent of `c` is a 3-child node, contract it
/// and propagate side-effect violations. Returns `true` if the contraction
/// resolved `c`'s violation (caller should return `None` immediately).
fn resolve_try_contract_parent<V: Accumulator>(
    vnodes: &mut Arena<VNode<V>>,
    p: VNodeId,
    c: VNodeId,
    violations: &mut Vec<VNodeId>,
) -> bool {
    if structural_child_count(vnodes, p) == 3 {
        tracing::debug!(p = %Nd(vnodes, p), "phase 1: contracting 3-node parent");
        let merged = contract(vnodes, p);
        push_side_effect_violations(vnodes, p, violations);
        push_side_effect_violations(vnodes, merged, violations);
        push_contraction_child_violations(vnodes, p, c, violations);
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
    vnodes: &mut Arena<VNode<V>>,
    gnodes: &mut Arena<GNode<C, V>>,
    c: VNodeId,
    p: VNodeId,
    g: VNodeId,
    violations: &mut Vec<VNodeId>,
    depth_evict: u32,
) -> Option<GNodeId> {
    // Optional grandparent contraction before the promote attempt.
    let g_merged = if structural_child_count(vnodes, g) == 3 {
        let merged = contract(vnodes, g);
        push_side_effect_violations(vnodes, g, violations);
        push_side_effect_violations(vnodes, merged, violations);
        push_promoted_violations(vnodes, g, violations);
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

    let result = if is_semi && v_depth(vnodes, c) <= depth_evict {
        tracing::debug!("phase 2: legacy promote (semi-internal entry)");
        let new_g = legacy_promote(vnodes, gnodes, c);
        push_side_effect_violations(vnodes, p, violations);
        Some(new_g)
    } else {
        skip_promote(vnodes, c);
        None
    };

    push_side_effect_violations(vnodes, g_id, violations);
    push_promoted_violations(vnodes, g_id, violations);
    if let Some(merged) = g_merged {
        push_source_10_violations(vnodes, merged, violations);
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
    vnodes: &mut Arena<VNode<V>>,
    gnodes: &mut Arena<GNode<C, V>>,
    c: VNodeId,
    violations: &mut Vec<VNodeId>,
    depth_evict: u32,
) -> Option<GNodeId> {
    let _span = tracing::debug_span!("resolve", node = c.index()).entered();
    tracing::debug!(ctx = %Ctx(vnodes, c), "begin");

    let Some(p) = vnodes.get(c.index()).parent() else {
        tracing::trace!("no parent — nothing to resolve");
        return None;
    };

    // Phase 1: optional parent contraction.
    if resolve_try_contract_parent(vnodes, p, c, violations) {
        return None;
    }

    // Path A: standard promote.
    if vnodes.get(c.index()).is_structural_pair() {
        tracing::debug!("phase 2: standard promote");
        standard_promote(vnodes, c);
        push_side_effect_violations(vnodes, p, violations);
        push_promoted_violations(vnodes, p, violations);
        escalate_after_promote(vnodes, p, violations);
        return None;
    }

    // Path B: skip / legacy promote.
    tracing::debug!("phase 2: skip promote path");
    let Some(g) = vnodes.get(p.index()).parent() else {
        tracing::trace!("no grandparent — cannot skip-promote");
        return None;
    };

    let result = resolve_path_b(vnodes, gnodes, c, p, g, violations, depth_evict);

    if vnodes.is_occupied(c.index()) && is_violated(vnodes, c) {
        tracing::warn!(
            node = %Ctx(vnodes, c),
            "resolve() returning with node STILL violated",
        );
    }

    result
}
