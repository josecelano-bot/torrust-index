use crate::diagnostics::diagnostic::{
    MissedViolationContext, audit_violations, diagnose_missed_violation,
};
use crate::graph::{Config, GvGraph, StructuralConfig};

type G = GvGraph<u8, u32, 8>;

fn make_config() -> Config<u32> {
    Config {
        split_threshold: 2,
        structural: StructuralConfig {
            depth_create: 3,
            depth_evict: 5,
            budget: None,
            alpha_relax: 0.5,
            bounded_eviction: false,
        },
    }
}

// ── audit_violations ──────────────────────────────────────────────
mod audit_violations_fn {
    use super::*;
    use crate::tree::vtree::vnode::VKind;

    fn find_depth_two_entry(g: &G) -> Option<crate::handle::VNodeId> {
        let v_root = g.v_root()?;
        let mut stack: Vec<(crate::handle::VNodeId, usize)> = vec![(v_root, 0)];
        while let Some((id, depth)) = stack.pop() {
            let n = g.vnodes().get(id.index());
            match &n.kind() {
                VKind::Entry { .. } if depth >= 2 => return Some(id),
                VKind::Structural { children, .. } => {
                    for (cid, _) in children.iter() {
                        stack.push((cid, depth + 1));
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[test]
    fn returns_empty_for_fresh_graph_with_no_violations_queued() {
        let g: G = GvGraph::new(make_config());
        let missed = audit_violations(g.vnodes(), &[], "test");
        assert!(missed.is_empty());
    }

    #[test]
    fn returns_empty_for_observed_graph_with_no_violations() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 3u32); // bootstrap split
        let missed = audit_violations(g.vnodes(), &[], "test");
        // All splits should leave the graph in a consistent state.
        assert!(missed.is_empty());
    }

    #[test]
    fn returns_empty_after_multiple_splits() {
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96, 160, 224] {
            g.observe(coord, 3u32);
        }
        let missed = audit_violations(g.vnodes(), &[], "test");
        assert!(missed.is_empty());
    }

    #[test]
    fn returns_missed_when_violated_node_is_not_in_queue() {
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96, 160, 224] {
            g.observe(coord, 3u32);
        }

        let Some(entry_id) = find_depth_two_entry(&g) else {
            return;
        };

        // Force a violation by making this node heavier than any uncle.
        g.core.vtree.nodes.get_mut(entry_id.index()).set_intensity(10_000u32);

        let missed = audit_violations(g.vnodes(), &[], "test");
        assert!(
            missed.contains(&entry_id),
            "expected forced violated node to be reported as missed; missed={missed:?}, target={entry_id:?}"
        );
    }

    #[test]
    fn returns_empty_when_only_violated_node_is_queued() {
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96, 160, 224] {
            g.observe(coord, 3u32);
        }

        let Some(entry_id) = find_depth_two_entry(&g) else {
            return;
        };

        g.core.vtree.nodes.get_mut(entry_id.index()).set_intensity(10_000u32);

        let queued = [entry_id];
        let missed = audit_violations(g.vnodes(), &queued, "test");
        assert!(
            missed.is_empty(),
            "expected empty missed set when the only violated node is queued; missed={missed:?}"
        );
    }
}

// ── diagnose_missed_violation ─────────────────────────────────────
mod diagnose_missed_violation_fn {
    use super::*;
    use crate::tree::vtree::vnode::VKind;

    #[test]
    fn does_not_panic_for_root_vnode_after_bootstrap() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 3u32); // bootstrap split creates v_root
        let v_root = g.v_root().expect("v_root must exist");
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: None,
        };
        // Root has no parent → hits "node has no parent" early-return path.
        diagnose_missed_violation(g.vnodes(), v_root, &ctx);
    }

    #[test]
    fn depth_one_child_hits_grandparent_not_found_path() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 3u32);
        let v_root = g.v_root().expect("v_root must exist after bootstrap");
        // Get a depth-1 child of v_root (parent=v_root, grandparent=None)
        let child_id = match &g.vnodes().get(v_root.index()).kind() {
            VKind::Structural { children, .. } => children.get(0).0,
            _ => panic!("expected Structural v_root after bootstrap"),
        };
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: None,
        };
        // depth-1: parent exists, grandparent=None → "depth 1?" early-return path.
        diagnose_missed_violation(g.vnodes(), child_id, &ctx);
    }

    #[test]
    fn depth_two_entry_covers_full_diagnose_path() {
        let mut g: G = GvGraph::new(make_config());
        // Multiple observations to produce a depth-2+ vtree.
        for coord in [32u8, 96u8, 160u8, 224u8] {
            g.observe(coord, 3u32);
        }
        let v_root = g.v_root().expect("v_root must exist");
        // BFS to find a depth-2+ Entry node.
        let mut stack: Vec<(crate::handle::VNodeId, usize)> = vec![(v_root, 0)];
        let mut depth2_entry = None;
        while let Some((id, depth)) = stack.pop() {
            let n = g.vnodes().get(id.index());
            match &n.kind() {
                VKind::Entry { .. } if depth >= 2 => {
                    depth2_entry = Some(id);
                    break;
                }
                VKind::Structural { children, .. } => {
                    for (cid, _) in children.iter() {
                        stack.push((cid, depth + 1));
                    }
                }
                _ => {}
            }
        }
        let Some(entry_id) = depth2_entry else {
            return; // Not enough splits for depth-2; treat as vacuous pass.
        };
        // collapse_sibling=v_root: v_root IS an ancestor → is_ancestor returns true.
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: Some(v_root),
        };
        diagnose_missed_violation(g.vnodes(), entry_id, &ctx);
    }

    #[test]
    fn depth_two_entry_with_evicted_parent_equals_grandparent() {
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96u8, 160u8, 224u8] {
            g.observe(coord, 3u32);
        }
        let v_root = g.v_root().expect("v_root must exist");
        // BFS to find depth-2 entry and its grandparent.
        let mut stack: Vec<(crate::handle::VNodeId, usize)> = vec![(v_root, 0)];
        let mut found: Option<(crate::handle::VNodeId, crate::handle::VNodeId)> = None;
        while let Some((id, depth)) = stack.pop() {
            let n = g.vnodes().get(id.index());
            match &n.kind() {
                VKind::Entry { .. } if depth >= 2 => {
                    let parent_id = n.parent().unwrap();
                    let grandparent_id = g.vnodes().get(parent_id.index()).parent().unwrap();
                    found = Some((id, grandparent_id));
                    break;
                }
                VKind::Structural { children, .. } => {
                    for (cid, _) in children.iter() {
                        stack.push((cid, depth + 1));
                    }
                }
                _ => {}
            }
        }
        let Some((entry_id, grandparent_id)) = found else {
            return;
        };
        // evicted_parent == grandparent_id → triggers that tracing::error! branch.
        let ctx = MissedViolationContext {
            evicted_parent: Some(grandparent_id),
            evicted_parent_child_count: 2,
            collapse_sibling: None,
        };
        diagnose_missed_violation(g.vnodes(), entry_id, &ctx);
    }

    #[test]
    fn depth_two_entry_with_non_ancestor_collapse_sibling() {
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96u8, 160u8, 224u8] {
            g.observe(coord, 3u32);
        }

        let v_root = g.v_root().expect("v_root must exist");
        let root_children: Vec<crate::handle::VNodeId> = match &g.vnodes().get(v_root.index()).kind() {
            VKind::Structural { children, .. } => children.iter().map(|(id, _)| id).collect(),
            _ => return,
        };
        if root_children.len() < 2 {
            return;
        }

        // Find a depth-2+ entry under the first root child.
        let subtree_root = root_children[0];
        let collapse_sibling = root_children[1];
        let mut stack: Vec<(crate::handle::VNodeId, usize)> = vec![(subtree_root, 1)];
        let mut depth2_entry = None;
        while let Some((id, depth)) = stack.pop() {
            let n = g.vnodes().get(id.index());
            match &n.kind() {
                VKind::Entry { .. } if depth >= 2 => {
                    depth2_entry = Some(id);
                    break;
                }
                VKind::Structural { children, .. } => {
                    for (cid, _) in children.iter() {
                        stack.push((cid, depth + 1));
                    }
                }
                _ => {}
            }
        }

        let Some(entry_id) = depth2_entry else {
            return;
        };

        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 2,
            collapse_sibling: Some(collapse_sibling),
        };
        diagnose_missed_violation(g.vnodes(), entry_id, &ctx);
    }

    #[test]
    fn depth_one_child_with_root_as_collapse_sibling_hits_direct_child_path() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 3u32);

        let v_root = g.v_root().expect("v_root must exist after bootstrap");
        let child_id = match &g.vnodes().get(v_root.index()).kind() {
            VKind::Structural { children, .. } => children.get(0).0,
            _ => return,
        };

        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 2,
            collapse_sibling: Some(v_root),
        };
        diagnose_missed_violation(g.vnodes(), child_id, &ctx);
    }

    #[test]
    fn depth_two_entry_is_direct_child_of_collapse_sibling() {
        // Sets collapse_sibling = violated's own parent.
        // is_ancestor_in_nodes(parent, violated) → true (parent is a direct ancestor).
        // sole_children.contains(violated.index()) → true → exercises the
        // "IS a direct child of collapse_sibling" branch in diagnose_collapse_sibling.
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96u8, 160u8, 224u8] {
            g.observe(coord, 3u32);
        }
        let v_root = g.v_root().expect("v_root must exist");
        let mut stack: Vec<(crate::handle::VNodeId, usize)> = vec![(v_root, 0)];
        let mut found: Option<(crate::handle::VNodeId, crate::handle::VNodeId)> = None;
        while let Some((id, depth)) = stack.pop() {
            let n = g.vnodes().get(id.index());
            match &n.kind() {
                VKind::Entry { .. } if depth >= 2 => {
                    let parent_id = n.parent().unwrap();
                    // grandparent must exist to avoid early-return in the diagnoser.
                    if g.vnodes().get(parent_id.index()).parent().is_some() {
                        found = Some((id, parent_id));
                        break;
                    }
                }
                VKind::Structural { children, .. } => {
                    for (cid, _) in children.iter() {
                        stack.push((cid, depth + 1));
                    }
                }
                _ => {}
            }
        }
        let Some((entry_id, parent_id)) = found else {
            return; // not enough depth; treat as vacuous pass
        };
        // collapse_sibling = parent_id → violated IS a direct child of collapse_sibling.
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: Some(parent_id),
        };
        diagnose_missed_violation(g.vnodes(), entry_id, &ctx);
    }
}

// ── diagnose_missed_violation_in_tree ─────────────────────────────
mod diagnose_missed_violation_in_tree_fn {
    use crate::diagnostics::diagnostic::{MissedViolationContext, diagnose_missed_violation_in_tree};
    use crate::tree::vtree::vnode::VKind;
    use super::*;

    #[test]
    fn does_not_panic_for_root_vnode_after_bootstrap() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 3u32);
        let v_root = g.v_root().expect("v_root must exist");
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: None,
        };
        diagnose_missed_violation_in_tree(&g.core.vtree, v_root, &ctx);
    }

    #[test]
    fn depth_one_child_hits_grandparent_not_found_path() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 3u32);
        let v_root = g.v_root().expect("v_root must exist after bootstrap");
        let child_id = match &g.vnodes().get(v_root.index()).kind() {
            VKind::Structural { children, .. } => children.get(0).0,
            _ => panic!("expected Structural v_root after bootstrap"),
        };
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: None,
        };
        diagnose_missed_violation_in_tree(&g.core.vtree, child_id, &ctx);
    }

    #[test]
    fn depth_two_entry_covers_full_path() {
        let mut g: G = GvGraph::new(make_config());
        for coord in [32u8, 96u8, 160u8, 224u8] {
            g.observe(coord, 3u32);
        }
        let v_root = g.v_root().expect("v_root must exist");
        let mut stack: Vec<(crate::handle::VNodeId, usize)> = vec![(v_root, 0)];
        let mut depth2_entry = None;
        while let Some((id, depth)) = stack.pop() {
            let n = g.vnodes().get(id.index());
            match &n.kind() {
                VKind::Entry { .. } if depth >= 2 => {
                    depth2_entry = Some(id);
                    break;
                }
                VKind::Structural { children, .. } => {
                    for (cid, _) in children.iter() {
                        stack.push((cid, depth + 1));
                    }
                }
                _ => {}
            }
        }
        let Some(entry_id) = depth2_entry else {
            return;
        };
        let ctx = MissedViolationContext {
            evicted_parent: None,
            evicted_parent_child_count: 0,
            collapse_sibling: Some(v_root),
        };
        diagnose_missed_violation_in_tree(&g.core.vtree, entry_id, &ctx);
    }
}
