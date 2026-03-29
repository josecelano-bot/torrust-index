use crate::arena::Arena;
use crate::handle::VNodeId;
use crate::nodes::vnode::{VKind, VNode};
use crate::traits::{Accumulator, Inspectable};
use crate::tree::vtree::VTree;

use super::{
    MissedViolationContext, diagnose_collapse_sibling, diagnose_collapse_sibling_in_tree,
    log_vtree_ancestry, log_vtree_ancestry_in_tree,
};

pub(super) fn diagnose_missed_violation_impl<V: Accumulator + Inspectable>(
    vnodes: &Arena<VNode<V>>,
    violated: VNodeId,
    context: &MissedViolationContext,
) {
    let v = vnodes.get(violated.index());
    let v_intensity = v.intensity();

    let Some(parent_id) = v.parent() else {
        tracing::error!(
            node = violated.index(),
            "DIAGNOSIS: node has no parent (root?), should not be violated",
        );
        return;
    };

    let parent = vnodes.get(parent_id.index());
    let Some(grandparent_id) = parent.parent() else {
        tracing::error!(
            node = violated.index(),
            parent = parent_id.index(),
            "DIAGNOSIS: parent has no grandparent (depth 1?), should not be violated",
        );
        return;
    };

    let grandparent = vnodes.get(grandparent_id.index());
    let uncles: Vec<(VNodeId, V)> = match &grandparent.kind() {
        VKind::Structural { children, .. } => {
            children.iter().filter(|(id, _)| *id != parent_id).collect()
        }
        VKind::Entry { .. } => vec![],
    };

    let max_uncle_intensity = uncles
        .iter()
        .map(|(_, int)| int.to_f64_approx())
        .fold(0.0_f64, f64::max);

    let uncle_desc: Vec<String> = uncles
        .iter()
        .map(|(id, int)| format!("v{}({})", id.index(), int.to_f64_approx()))
        .collect();

    tracing::error!(
        node = violated.index(),
        intensity = v_intensity.to_f64_approx(),
        parent = parent_id.index(),
        parent_intensity = parent.intensity().to_f64_approx(),
        grandparent = grandparent_id.index(),
        grandparent_intensity = grandparent.intensity().to_f64_approx(),
        ?uncle_desc,
        max_uncle = max_uncle_intensity,
        is_violation = v_intensity.to_f64_approx() > max_uncle_intensity,
        "MISSED VIOLATION DIAGNOSIS",
    );

    tracing::error!(
        evicted_parent = ?context.evicted_parent.map(VNodeId::index),
        child_count = context.evicted_parent_child_count,
        collapse_sibling = ?context.collapse_sibling.map(VNodeId::index),
        "eviction context (child_count: 2=collapse, 3=3→2)",
    );

    if let Some(sole) = context.collapse_sibling {
        diagnose_collapse_sibling(vnodes, violated, sole);
    }

    if let Some(evicted_p) = context.evicted_parent {
        if evicted_p == grandparent_id {
            tracing::error!(
                node = violated.index(),
                evicted_parent = evicted_p.index(),
                "node's grandparent = evicted_parent → parent is sibling of removed entry. Check source 8.",
            );
        }
    }

    log_vtree_ancestry(vnodes, violated);
}

pub(super) fn diagnose_missed_violation_in_tree_impl<V: Accumulator + Inspectable>(
    vtree: &VTree<V>,
    violated: VNodeId,
    context: &MissedViolationContext,
) {
    let vnodes = &vtree.nodes;
    let v = vnodes.get(violated.index());
    let v_intensity = v.intensity();

    let Some(parent_id) = v.parent() else {
        tracing::error!(
            node = violated.index(),
            "DIAGNOSIS: node has no parent (root?), should not be violated",
        );
        return;
    };

    let parent = vnodes.get(parent_id.index());
    let Some(grandparent_id) = parent.parent() else {
        tracing::error!(
            node = violated.index(),
            parent = parent_id.index(),
            "DIAGNOSIS: parent has no grandparent (depth 1?), should not be violated",
        );
        return;
    };

    let grandparent = vnodes.get(grandparent_id.index());
    let uncles: Vec<(VNodeId, V)> = match &grandparent.kind() {
        VKind::Structural { children, .. } => {
            children.iter().filter(|(id, _)| *id != parent_id).collect()
        }
        VKind::Entry { .. } => vec![],
    };

    let max_uncle_intensity = uncles
        .iter()
        .map(|(_, int)| int.to_f64_approx())
        .fold(0.0_f64, f64::max);

    let uncle_desc: Vec<String> = uncles
        .iter()
        .map(|(id, int)| format!("v{}({})", id.index(), int.to_f64_approx()))
        .collect();

    tracing::error!(
        node = violated.index(),
        intensity = v_intensity.to_f64_approx(),
        parent = parent_id.index(),
        parent_intensity = parent.intensity().to_f64_approx(),
        grandparent = grandparent_id.index(),
        grandparent_intensity = grandparent.intensity().to_f64_approx(),
        ?uncle_desc,
        max_uncle = max_uncle_intensity,
        is_violation = v_intensity.to_f64_approx() > max_uncle_intensity,
        "MISSED VIOLATION DIAGNOSIS",
    );

    tracing::error!(
        evicted_parent = ?context.evicted_parent.map(VNodeId::index),
        child_count = context.evicted_parent_child_count,
        collapse_sibling = ?context.collapse_sibling.map(VNodeId::index),
        "eviction context (child_count: 2=collapse, 3=3→2)",
    );

    if let Some(sole) = context.collapse_sibling {
        diagnose_collapse_sibling_in_tree(vtree, violated, sole);
    }

    if let Some(evicted_p) = context.evicted_parent {
        if evicted_p == grandparent_id {
            tracing::error!(
                node = violated.index(),
                evicted_parent = evicted_p.index(),
                "node's grandparent = evicted_parent → parent is sibling of removed entry. Check source 8.",
            );
        }
    }

    log_vtree_ancestry_in_tree(vtree, violated);
}
