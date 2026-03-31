use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::VKind;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GTree;
use crate::tree::vtree::VNodeTree;

use super::super::promote::{legacy_promote, skip_promote, standard_promote};
use super::super::violation_push::ViolationQueue;
use super::{Ctx, EscalationContext, Nd, VTreeMutContext, contract, is_violated};

fn structural_child_count<V: Accumulator>(vnodes: &VNodeTree<V>, id: VNodeId) -> usize {
    vnodes.get(id.index()).child_count()
}

fn any_child_violated<V: Accumulator>(vnodes: &VNodeTree<V>, node: VNodeId) -> bool {
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

fn v_depth_local<V: Accumulator>(vnodes: &VNodeTree<V>, id: VNodeId) -> u32 {
    vnodes.depth(id)
}

/// Contracts the 3-child parent `p` after a promote, propagates violations,
/// and returns `Some(merged)` if the violation persists (Phase 3 needed),
/// or `None` if the contraction resolved it.
fn escalate_contract_parent<V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    ctx: &mut EscalationContext,
) -> Option<VNodeId> {
    let merged = contract(tree.vtree, ctx.parent_id);
    {
        let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, ctx.parent_id);
        queue.push_side_effect(vnodes, merged);
        queue.push_contraction_child(vnodes, ctx.parent_id, ctx.heaviest_id);
    }
    ctx.merged_id = Some(merged);

    let needs_skip = if ctx.heaviest_is_direct_child {
        is_violated(&tree.vtree.nodes, ctx.heaviest_id)
    } else {
        is_violated(&tree.vtree.nodes, merged)
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
    if structural_child_count(&tree.vtree.nodes, ctx.grandparent_id) == 3 {
        let g_merged = contract(tree.vtree, ctx.grandparent_id);
        {
            let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
            let mut queue = ViolationQueue::new(violations);
            queue.push_side_effect(vnodes, ctx.grandparent_id);
            queue.push_side_effect(vnodes, g_merged);
            queue.push_promoted(vnodes, ctx.grandparent_id);
        }
        ctx.grandparent_merged_id = Some(g_merged);

        let merged = ctx
            .merged_id
            .expect("escalate_try_contract_grandparent: parent contraction must run first");
        let resolved = !is_violated(&tree.vtree.nodes, ctx.heaviest_id)
            && (ctx.heaviest_is_direct_child || !is_violated(&tree.vtree.nodes, merged));
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
    let g_id_opt = tree.vtree.nodes.get(ctx.parent_id.index()).parent();
    if let Some(g_id) = g_id_opt {
        skip_promote(tree.vtree, ctx.heaviest_id);
        let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, g_id);
        queue.push_promoted(vnodes, g_id);

        if let Some(gm) = ctx.grandparent_merged_id {
            queue.push_source_10(vnodes, gm);
        }
    }
}

fn escalate_after_promote<V: Accumulator>(tree: &mut VTreeMutContext<'_, V>, p: VNodeId) {
    // Phase 1: Identify heaviest child; early-return if no violation.
    // Scope the shared borrow so it is dropped before the mutable helper calls.
    let (heaviest, h_direct, g) = {
        let vnodes = &tree.vtree.nodes;
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
        (heaviest, h_direct, g)
    }; // shared borrow of tree.vtree.nodes dropped here

    let _span = tracing::debug_span!(
        "escalate",
        h = %Nd(&tree.vtree.nodes, heaviest),
        reason = if h_direct { "direct" } else { "indirect" },
    )
    .entered();

    let mut ctx = EscalationContext::new(p, g, heaviest, h_direct);

    // Phase 2: Contract 3-child parent `p` and propagate violations.
    let Some(_merged) = escalate_contract_parent(tree, &mut ctx) else {
        return;
    };

    // Phase 3: Optionally contract grandparent `g` if it has 3 children.
    if escalate_try_contract_grandparent(tree, &mut ctx) {
        return;
    }

    // Phase 4: Skip-promote fallback.
    escalate_skip_promote(tree, &ctx);
}

/// Phase 1 of `resolve`: if the parent of `c` is a 3-child node, contract it
/// and propagate side-effect violations. Returns `true` if the contraction
/// resolved `c`'s violation (caller should return `None` immediately).
fn resolve_try_contract_parent<V: Accumulator>(
    tree: &mut VTreeMutContext<'_, V>,
    p: VNodeId,
    c: VNodeId,
) -> bool {
    if structural_child_count(&tree.vtree.nodes, p) == 3 {
        tracing::debug!(p = %Nd(&tree.vtree.nodes, p), "phase 1: contracting 3-node parent");
        let merged = contract(tree.vtree, p);
        {
            let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
            let mut queue = ViolationQueue::new(violations);
            queue.push_side_effect(vnodes, p);
            queue.push_side_effect(vnodes, merged);
            queue.push_contraction_child(vnodes, p, c);
        }
        if !is_violated(&tree.vtree.nodes, c) {
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
fn resolve_path_b<C: Coordinate, V: Accumulator, const N: u32>(
    tree: &mut VTreeMutContext<'_, V>,
    gtree: &mut GTree<C, V, N>,
    c: VNodeId,
    p: VNodeId,
    g: VNodeId,
    depth_evict: u32,
) -> Option<GNodeId> {
    // Optional grandparent contraction before the promote attempt.
    let g_merged = if structural_child_count(&tree.vtree.nodes, g) == 3 {
        let merged = contract(tree.vtree, g);
        {
            let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
            let mut queue = ViolationQueue::new(violations);
            queue.push_side_effect(vnodes, g);
            queue.push_side_effect(vnodes, merged);
            queue.push_promoted(vnodes, g);
        }
        if !is_violated(&tree.vtree.nodes, c) {
            tracing::debug!("phase 2: resolved by g-contraction");
            return None;
        }
        Some(merged)
    } else {
        None
    };

    let Some(g_id) = tree.vtree.nodes.get(p.index()).parent() else {
        tracing::warn!(
            node = %Ctx(&tree.vtree.nodes, c),
            "skip-promote path: no grandparent after g-contraction — resolve incomplete",
        );
        return None;
    };

    let is_semi = matches!(
        &tree.vtree.nodes.get(c.index()).kind(),
        VKind::Entry { gnode, .. }
            if gtree.nodes.get(gnode.index()).is_semi_internal()
    );

    let result = if is_semi && v_depth_local(&tree.vtree.nodes, c) <= depth_evict {
        tracing::debug!("phase 2: legacy promote (semi-internal entry)");
        let new_g = legacy_promote(tree.vtree, gtree, c);
        {
            let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
            let mut queue = ViolationQueue::new(violations);
            queue.push_side_effect(vnodes, p);
        }
        Some(new_g)
    } else {
        skip_promote(tree.vtree, c);
        None
    };

    {
        let (vnodes, violations) = (&tree.vtree.nodes, &mut tree.vtree.violations);
        let mut queue = ViolationQueue::new(violations);
        queue.push_side_effect(vnodes, g_id);
        queue.push_promoted(vnodes, g_id);
        if let Some(merged) = g_merged {
            queue.push_source_10(vnodes, merged);
        }
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
pub fn resolve<C: Coordinate, V: Accumulator, const N: u32>(
    tree: &mut VTreeMutContext<'_, V>,
    gtree: &mut GTree<C, V, N>,
    c: VNodeId,
    depth_evict: u32,
) -> Option<GNodeId> {
    let _span = tracing::debug_span!("resolve", node = c.index()).entered();
    tracing::debug!(ctx = %Ctx(&tree.vtree.nodes, c), "begin");

    let Some(p) = tree.vtree.nodes.get(c.index()).parent() else {
        tracing::trace!("no parent — nothing to resolve");
        return None;
    };

    // Phase 1: optional parent contraction.
    if resolve_try_contract_parent(tree, p, c) {
        return None;
    }

    // Path A: standard promote.
    if tree.vtree.nodes.get(c.index()).is_structural_pair() {
        tracing::debug!("phase 2: standard promote");
        standard_promote(tree.vtree, c);
        let mut queue = ViolationQueue::new(&mut tree.vtree.violations);
        queue.push_side_effect(&tree.vtree.nodes, p);
        queue.push_promoted(&tree.vtree.nodes, p);
        escalate_after_promote(tree, p);
        return None;
    }

    // Path B: skip / legacy promote.
    tracing::debug!("phase 2: skip promote path");
    let Some(g) = tree.vtree.nodes.get(p.index()).parent() else {
        tracing::trace!("no grandparent — cannot skip-promote");
        return None;
    };

    let result = resolve_path_b(tree, gtree, c, p, g, depth_evict);

    if tree.vtree.nodes.is_occupied(c.index()) && is_violated(&tree.vtree.nodes, c) {
        tracing::warn!(
            node = %Ctx(&tree.vtree.nodes, c),
            "resolve() returning with node STILL violated",
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use crate::graph::algorithm::rebalance::VTreeMutContext;
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::handle::{GNodeId, VNodeId};
    use crate::nodes::vnode::{Children, VKind, VNode};

    type G = GvGraph<u8, u32, 8>;

    fn make_graph() -> G {
        GvGraph::new(Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 3,
                depth_evict: 5,
                budget: None,
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        })
    }

    #[test]
    fn resolve_returns_none_when_node_has_no_parent() {
        let mut g = make_graph();
        let c = g.v_root().expect("fresh graph must have v_root");
        let depth_evict = g.core.gtree.live_depth_evict;
        let mut tree = VTreeMutContext {
            vtree: &mut g.core.vtree,
        };

        let out = resolve(&mut tree, &mut g.core.gtree, c, depth_evict);
        assert!(out.is_none());
    }

    #[test]
    fn resolve_skip_path_returns_none_without_grandparent() {
        let mut g = make_graph();
        g.observe(64u8, 3u32);

        let v_root = g.v_root().expect("v_root should exist after split");
        let c = match g.core.vtree.nodes.get(v_root.index()).kind() {
            VKind::Structural { children, .. } => children.get(0).0,
            VKind::Entry { .. } => panic!("expected structural v_root after split"),
        };

        let depth_evict = g.core.gtree.live_depth_evict;
        let mut tree = VTreeMutContext {
            vtree: &mut g.core.vtree,
        };
        let out = resolve(&mut tree, &mut g.core.gtree, c, depth_evict);
        assert!(out.is_none());
    }

    #[test]
    fn resolve_path_b_legacy_promote_returns_new_gnode() {
        let mut g = make_graph();

        let semi_gid = g.core.gtree.nodes.root;
        let existing_child = g.core.gtree.nodes.allocate_missing_child(semi_gid);

        let c = VNodeId::from_index(
            g.core
                .vtree
                .nodes
                .alloc(VNode::new_entry(7, None, semi_gid, true, true))
                .0,
        );
        let s = VNodeId::from_index(
            g.core
                .vtree
                .nodes
                .alloc(VNode::new_entry(
                    5,
                    None,
                    GNodeId::from_index(existing_child.index()),
                    true,
                    true,
                ))
                .0,
        );
        let u = VNodeId::from_index(
            g.core
                .vtree
                .nodes
                .alloc(VNode::new_entry(
                    11,
                    None,
                    GNodeId::from_index(existing_child.index()),
                    true,
                    true,
                ))
                .0,
        );

        let p = VNodeId::from_index(
            g.core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    12,
                    None,
                    Children::new_2((c, 7), (s, 5)),
                    true,
                ))
                .0,
        );
        g.core.vtree.nodes.get_mut(c.index()).set_parent(p);
        g.core.vtree.nodes.get_mut(s.index()).set_parent(p);

        let gp = VNodeId::from_index(
            g.core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    23,
                    None,
                    Children::new_2((p, 12), (u, 11)),
                    true,
                ))
                .0,
        );
        g.core.vtree.nodes.get_mut(p.index()).set_parent(gp);
        g.core.vtree.nodes.get_mut(u.index()).set_parent(gp);

        g.core.gtree.nodes.assign_entry(semi_gid, c);

        let depth_evict = g.core.gtree.live_depth_evict;
        let mut tree = VTreeMutContext {
            vtree: &mut g.core.vtree,
        };
        let out = resolve(&mut tree, &mut g.core.gtree, c, depth_evict);

        let new_gid = out.expect("legacy promote path should return new gnode");
        assert_eq!(
            g.core.gtree.nodes.get(new_gid.index()).parent(),
            Some(semi_gid)
        );
    }
}
