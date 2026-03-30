use crate::handle::VNodeId;
use crate::nodes::vnode::VKind;
use crate::traits::{Accumulator, Inspectable};
use crate::tree::vtree::{VNodeTree, VTree};

fn is_ancestor_in_nodes<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    ancestor: VNodeId,
    mut descendant: VNodeId,
) -> bool {
    while let Some(p) = vnodes.get(descendant.index()).parent() {
        if p == ancestor {
            return true;
        }
        descendant = p;
    }
    false
}

#[allow(clippy::too_many_lines)]
/// Walks the V-tree ancestry from `violated` to the root, logging each hop
/// at `tracing::error!` level for post-mortem diagnosis.
pub(super) fn log_vtree_ancestry<V: Accumulator + Inspectable>(
    vnodes: &VNodeTree<V>,
    violated: VNodeId,
) {
    let _span =
        tracing::error_span!("vtree_path_to_violated", node = violated.index()).entered();
    let mut current = violated;
    let mut depth = 0_usize;
    loop {
        let n = vnodes.get(current.index());
        let kind = match &n.kind() {
            VKind::Entry { .. } => "E",
            VKind::Structural { children, .. } => match children.len() {
                2 => "S2",
                3 => "S3",
                _ => "S?",
            },
        };
        tracing::error!(
            depth,
            node = current.index(),
            kind,
            intensity = n.intensity().to_f64_approx(),
            "ancestry",
        );
        match n.parent() {
            Some(p) => {
                current = p;
                depth += 1;
            }
            None => break,
        }
    }
}

/// Emits diagnostics for the `collapse_sibling` context field: checks whether
/// `violated` is a descendant of `sole` and explains which source should have
/// caught the violation.
pub(super) fn diagnose_collapse_sibling<V: Accumulator + Inspectable>(
    vnodes: &VNodeTree<V>,
    violated: VNodeId,
    sole: VNodeId,
) {
    if is_ancestor_in_nodes(vnodes, sole, violated) {
        tracing::error!(
            node = violated.index(),
            collapse_sibling = sole.index(),
            "node IS a descendant of collapse_sibling → should have been caught by source 7",
        );

        let sole_children: Vec<usize> = match &vnodes.get(sole.index()).kind() {
            VKind::Structural { children, .. } => (0..children.len())
                .map(|i| children.get(i).0.index())
                .collect(),
            VKind::Entry { .. } => vec![],
        };
        if sole_children.contains(&violated.index()) {
            tracing::error!(
                node = violated.index(),
                "node IS a direct child of collapse_sibling — source 7 should catch it",
            );
        } else {
            tracing::error!(
                node = violated.index(),
                ?sole_children,
                "node is NOT a direct child — source 7 only checks direct children. MISSING SOURCE.",
            );
        }
    } else {
        tracing::error!(
            node = violated.index(),
            collapse_sibling = sole.index(),
            "node is NOT a descendant of collapse_sibling",
        );
    }
}

pub(super) fn diagnose_collapse_sibling_in_tree<V: Accumulator + Inspectable>(
    vtree: &VTree<V>,
    violated: VNodeId,
    sole: VNodeId,
) {
    diagnose_collapse_sibling(&vtree.nodes, violated, sole);
}

pub(super) fn log_vtree_ancestry_in_tree<V: Accumulator + Inspectable>(
    vtree: &VTree<V>,
    violated: VNodeId,
) {
    log_vtree_ancestry(&vtree.nodes, violated);
}
