//! Violation-push helpers — all functions that push violated `VNodeId`s into
//! a `violations` vector based on structural conditions in the V-tree.
//!
//! These are a cohesive family unrelated to the rebalancing algorithm itself.
//! Callers in `rebalance.rs` import them via `use super::violation_push::*`.
//!
//! # Violation source numbering
//!
//! Each push function corresponds to a *violation source* — a structural
//! condition that can leave a V-node in a violated state after a mutation.
//! The sources are numbered from an original taxonomy; sources 1, 2, and 5
//! were merged into adjacent sources or no longer occur after later algorithm
//! revisions, which is why those numbers are absent.
//!
//! | Source | Name field in `ViolationSources`        | Trigger                                       |
//! |--------|-----------------------------------------|-----------------------------------------------|
//! | 3      | `source_3_contraction_grandchildren`    | Contraction shifts grandchildren to siblings  |
//! | 4      | `source_4_promotion_children`           | Promote exposes new children of the parent    |
//! | 6      | `source_6_leaf_removal_ancestors`       | Leaf removal propagates up ancestor chain     |
//! | 7      | `source_7_collapse_children`            | 2→1 collapse leaves the sole child violated   |
//! | 8      | `source_8_three_to_two_siblings`        | 3→2 sibling reduction after removal           |
//! | 9      | `source_9_collapse_cousins`             | Collapse exposes cousins via grandparent path |
//! | 10     | `source_10_g_contraction_promotion`     | G-contraction followed by promotion           |

use crate::handle::VNodeId;
use crate::nodes::vnode::VKind;
use crate::traits::Accumulator;
use crate::tree::vtree::VNodeTree;

use super::rebalance::{Nd, is_violated};
use super::violation_sources::ViolationSources;

pub struct ViolationQueue<'a> {
    violations: &'a mut Vec<VNodeId>,
}

impl<'a> ViolationQueue<'a> {
    #[must_use]
    pub const fn new(violations: &'a mut Vec<VNodeId>) -> Self {
        Self { violations }
    }

    pub fn push_side_effect<V: Accumulator>(&mut self, vnodes: &VNodeTree<V>, node: VNodeId) {
        push_side_effect_violations(vnodes, node, self.violations);
    }

    pub fn push_promoted<V: Accumulator>(&mut self, vnodes: &VNodeTree<V>, node: VNodeId) {
        push_promoted_violations(vnodes, node, self.violations);
    }

    pub fn push_contraction_child<V: Accumulator>(
        &mut self,
        vnodes: &VNodeTree<V>,
        node: VNodeId,
        skip: VNodeId,
    ) {
        push_contraction_child_violations(vnodes, node, skip, self.violations);
    }

    pub fn push_source_10<V: Accumulator>(&mut self, vnodes: &VNodeTree<V>, node: VNodeId) {
        push_source_10_violations(vnodes, node, self.violations);
    }

    pub fn push_leaf_removal<V: Accumulator>(&mut self, vnodes: &VNodeTree<V>, start: VNodeId) {
        push_leaf_removal_violations(vnodes, start, self.violations);
    }

    pub fn push_collapse<V: Accumulator>(&mut self, vnodes: &VNodeTree<V>, sole: VNodeId) {
        push_collapse_violations(vnodes, sole, self.violations);
    }

    pub fn push_remaining_sibling<V: Accumulator>(
        &mut self,
        vnodes: &VNodeTree<V>,
        parent: VNodeId,
        removed: VNodeId,
    ) {
        push_remaining_sibling_violations(vnodes, parent, removed, self.violations);
    }

    pub fn push_cousin<V: Accumulator>(
        &mut self,
        vnodes: &VNodeTree<V>,
        sole: VNodeId,
        grandparent: VNodeId,
    ) {
        push_cousin_violations(vnodes, sole, grandparent, self.violations);
    }
}

// ── Plain wrappers (use all-enabled config) ──────────────────────────────────

pub fn push_side_effect_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_side_effect_violations_with_config(
        vnodes,
        node,
        violations,
        ViolationSources::all_enabled(),
    );
}

pub fn push_source_10_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_source_10_violations_with_config(
        vnodes,
        node,
        violations,
        ViolationSources::all_enabled(),
    );
}

pub fn push_promoted_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_promoted_violations_with_config(vnodes, node, violations, ViolationSources::all_enabled());
}

pub fn push_contraction_child_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    skip: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    let child_ids: Vec<VNodeId> = match &vnodes.get(node.index()).kind() {
        VKind::Structural { children, .. } => children.iter().map(|(id, _)| id).collect(),
        VKind::Entry { .. } => return,
    };
    for child_id in child_ids {
        if child_id != skip && is_violated(vnodes, child_id) {
            tracing::trace!(
                child = %Nd(vnodes, child_id),
                skip = skip.index(),
                "contraction-child violation",
            );
            violations.push(child_id);
        }
    }
}

pub fn push_leaf_removal_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    start: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_leaf_removal_violations_with_config(
        vnodes,
        start,
        violations,
        ViolationSources::all_enabled(),
    );
}

pub fn push_collapse_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    sole: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_collapse_violations_with_config(vnodes, sole, violations, ViolationSources::all_enabled());
}

pub fn push_remaining_sibling_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    p: VNodeId,
    removed: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_remaining_sibling_violations_with_config(
        vnodes,
        p,
        removed,
        violations,
        ViolationSources::all_enabled(),
    );
}

pub fn push_cousin_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    sole: VNodeId,
    grandparent: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    push_cousin_violations_with_config(
        vnodes,
        sole,
        grandparent,
        violations,
        ViolationSources::all_enabled(),
    );
}

// ── Config variants ──────────────────────────────────────────────────────────

#[inline]
pub fn push_side_effect_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_3_contraction_grandchildren {
        return;
    }
    push_grandchild_violations(vnodes, node, violations);
}

#[inline]
pub fn push_source_10_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_10_g_contraction_promotion {
        return;
    }
    push_grandchild_violations(vnodes, node, violations);
}

#[inline]
pub fn push_promoted_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_4_promotion_children {
        return;
    }
    let child_ids: Vec<VNodeId> = match &vnodes.get(node.index()).kind() {
        VKind::Structural { children, .. } => children.iter().map(|(id, _)| id).collect(),
        VKind::Entry { .. } => return,
    };
    for child_id in child_ids {
        if is_violated(vnodes, child_id) {
            tracing::trace!(child = %Nd(vnodes, child_id), at = %Nd(vnodes, node), "promoted violation");
            violations.push(child_id);
        }
    }
}

#[inline]
pub fn push_leaf_removal_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    start: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_6_leaf_removal_ancestors {
        return;
    }
    let mut ancestor = start;
    while let Some(parent) = vnodes.get(ancestor.index()).parent() {
        let sibling_ids: Vec<VNodeId> = match &vnodes.get(parent.index()).kind() {
            VKind::Structural { children, .. } => children
                .iter()
                .map(|(id, _)| id)
                .filter(|&id| id != ancestor)
                .collect(),
            VKind::Entry { .. } => break,
        };

        for sib_id in sibling_ids {
            if let VKind::Structural { children, .. } = &vnodes.get(sib_id.index()).kind() {
                for i in 0..children.len() {
                    let (child_id, _) = children.get(i);
                    if is_violated(vnodes, child_id) {
                        tracing::trace!(
                            child = %Nd(vnodes, child_id),
                            weakened_uncle = %Nd(vnodes, ancestor),
                            "leaf-removal violation",
                        );
                        violations.push(child_id);
                    }
                }
            }
        }

        ancestor = parent;
    }
}

#[inline]
pub fn push_collapse_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    sole: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_7_collapse_children {
        return;
    }
    tracing::debug!(
        sole = sole.index(),
        "push_collapse_violations: checking node"
    );
    push_children_violations(vnodes, sole, "collapse (source 7)", violations);
}

#[inline]
pub fn push_remaining_sibling_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    p: VNodeId,
    removed: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_8_three_to_two_siblings {
        return;
    }

    let remaining: Vec<VNodeId> = match &vnodes.get(p.index()).kind() {
        VKind::Structural { children, .. } => children
            .iter()
            .map(|(id, _)| id)
            .filter(|&id| id != removed)
            .collect(),
        VKind::Entry { .. } => return,
    };

    for sibling in remaining {
        push_children_violations(vnodes, sibling, "3→2 transition (source 8)", violations);
    }
}

#[inline]
pub fn push_cousin_violations_with_config<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    sole: VNodeId,
    grandparent: VNodeId,
    violations: &mut Vec<VNodeId>,
    config: ViolationSources,
) {
    if !config.source_9_collapse_cousins {
        return;
    }

    let cousins: Vec<VNodeId> = match &vnodes.get(grandparent.index()).kind() {
        VKind::Structural { children, .. } => children
            .iter()
            .map(|(id, _)| id)
            .filter(|&id| id != sole)
            .collect(),
        VKind::Entry { .. } => return,
    };

    tracing::debug!(
        sole = sole.index(),
        grandparent = grandparent.index(),
        cousins = ?cousins.iter().map(|c| c.index()).collect::<Vec<_>>(),
        "push_cousin_violations: checking cousins' children (source 9)",
    );

    for cousin in cousins {
        push_children_violations(vnodes, cousin, "collapse cousins (source 9)", violations);
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

fn push_grandchild_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    violations: &mut Vec<VNodeId>,
) {
    let child_ids: Vec<VNodeId> = match &vnodes.get(node.index()).kind() {
        VKind::Structural { children, .. } => children.iter().map(|(id, _)| id).collect(),
        VKind::Entry { .. } => return,
    };
    for child_id in child_ids {
        if let VKind::Structural { children, .. } = &vnodes.get(child_id.index()).kind() {
            for i in 0..children.len() {
                let (gc_id, _) = children.get(i);
                if is_violated(vnodes, gc_id) {
                    tracing::trace!(gc = %Nd(vnodes, gc_id), at = %Nd(vnodes, node), "side-effect violation");
                    violations.push(gc_id);
                }
            }
        }
    }
}

fn push_children_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    node: VNodeId,
    source: &str,
    violations: &mut Vec<VNodeId>,
) {
    let children: Vec<VNodeId> = match &vnodes.get(node.index()).kind() {
        VKind::Structural { children, .. } => {
            (0..children.len()).map(|i| children.get(i).0).collect()
        }
        VKind::Entry { .. } => {
            tracing::debug!(
                node = node.index(),
                source,
                "push_children_violations: node is entry, no children",
            );
            return;
        }
    };

    tracing::debug!(
        node = node.index(),
        children = ?children.iter().map(|c| c.index()).collect::<Vec<_>>(),
        source,
        "push_children_violations: checking children",
    );

    for child_id in children {
        let violated = is_violated(vnodes, child_id);
        tracing::debug!(
            child = child_id.index(),
            violated,
            source,
            "push_children_violations: child check",
        );
        if violated {
            tracing::trace!(
                child = %Nd(vnodes, child_id),
                parent = %Nd(vnodes, node),
                source,
                "sibling-removal violation",
            );
            violations.push(child_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::handle::GNodeId;
    use crate::nodes::vnode::{Children, VNode};

    fn make_vnodes() -> VNodeTree<u32> {
        VNodeTree::from(Arena::new())
    }

    fn all_disabled() -> ViolationSources {
        ViolationSources {
            source_3_contraction_grandchildren: false,
            source_4_promotion_children: false,
            source_6_leaf_removal_ancestors: false,
            source_7_collapse_children: false,
            source_8_three_to_two_siblings: false,
            source_9_collapse_cousins: false,
            source_10_g_contraction_promotion: false,
        }
    }

    #[test]
    fn config_disabled_paths_do_not_push() {
        let mut vnodes = make_vnodes();
        let e1 = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
            1,
            None,
            GNodeId::from_index(1),
            true,
            true,
        )).0);
        let e2 = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
            2,
            None,
            GNodeId::from_index(2),
            true,
            true,
        )).0);
        let p = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
            3,
            None,
            Children::new_2((e1, 1), (e2, 2)),
            true,
        )).0);
        vnodes.get_mut(e1.index()).set_parent(p);
        vnodes.get_mut(e2.index()).set_parent(p);

        let mut violations = Vec::new();
        let cfg = all_disabled();

        push_side_effect_violations_with_config(&vnodes, p, &mut violations, cfg);
        push_source_10_violations_with_config(&vnodes, p, &mut violations, cfg);
        push_promoted_violations_with_config(&vnodes, p, &mut violations, cfg);
        push_leaf_removal_violations_with_config(&vnodes, p, &mut violations, cfg);
        push_collapse_violations_with_config(&vnodes, p, &mut violations, cfg);
        push_remaining_sibling_violations_with_config(&vnodes, p, e1, &mut violations, cfg);
        push_cousin_violations_with_config(&vnodes, e1, p, &mut violations, cfg);

        assert!(violations.is_empty());
    }

    #[test]
    fn entry_node_paths_return_without_pushes() {
        let mut vnodes = make_vnodes();
        let entry = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
            1,
            None,
            GNodeId::from_index(1),
            true,
            true,
        )).0);
        let other = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
            2,
            None,
            GNodeId::from_index(2),
            true,
            true,
        )).0);
        let mut violations = Vec::new();

        push_contraction_child_violations(&vnodes, entry, other, &mut violations);
        push_promoted_violations_with_config(
            &vnodes,
            entry,
            &mut violations,
            ViolationSources::all_enabled(),
        );
        push_remaining_sibling_violations_with_config(
            &vnodes,
            entry,
            other,
            &mut violations,
            ViolationSources::all_enabled(),
        );
        push_cousin_violations_with_config(
            &vnodes,
            entry,
            other,
            &mut violations,
            ViolationSources::all_enabled(),
        );
        push_grandchild_violations(&vnodes, entry, &mut violations);
        push_children_violations(&vnodes, entry, "entry-source", &mut violations);

        assert!(violations.is_empty());
    }
}
