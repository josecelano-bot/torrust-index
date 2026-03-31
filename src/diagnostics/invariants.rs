mod budget_checks;
mod graph_consistency;
mod reporting;
mod vtree_consistency;

use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::handle::{GNodeId, VNodeId};
use crate::tree::vtree::vnode::VKind;

use budget_checks::check_accounting_invariants;
use graph_consistency::check_g_tree_invariants;
pub use reporting::{assert_invariants, check_all_invariants};
#[cfg(feature = "dynamic-contour-tracking")]
pub use reporting::{check_p_i3_only, check_plateau_only};
use vtree_consistency::check_v_tree_invariants;

pub use crate::diagnostics::dot::{dump_gtree_dot, dump_vtree_dot};
pub use crate::diagnostics::dump::{dump_gtree, dump_plateaus};

#[allow(clippy::float_cmp)]
fn check_g_i1_summation<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, g) in graph.core.gtree.nodes.iter_occupied() {
        let left_sum = g.left().map_or(0.0, |l| {
            graph.core.gtree.nodes.get(l.index()).sum().to_f64_approx()
        });
        let right_sum = g.right().map_or(0.0, |r| {
            graph.core.gtree.nodes.get(r.index()).sum().to_f64_approx()
        });
        let expected = g.own().to_f64_approx() + left_sum + right_sum;
        let actual = g.sum().to_f64_approx();
        if expected != actual && (expected - actual).abs() > 1e-9 {
            errors.push(format!(
                "G-I1 violated at G-node {idx}: expected sum={expected}, actual sum={actual} \
                 (own={}, left_sum={left_sum}, right_sum={right_sum})",
                g.own().to_f64_approx()
            ));
        }
    }
}

fn check_g_i2_variable_fanout<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, g) in graph.core.gtree.nodes.iter_occupied() {
        let count = usize::from(g.left().is_some()) + usize::from(g.right().is_some());
        if count > 2 {
            errors.push(format!(
                "G-I2 violated at G-node {idx}: {count} children (max 2)"
            ));
        }
    }
}

fn check_g_i4_entry_consistency<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, g) in graph.core.gtree.nodes.iter_occupied() {
        if let Some(v_id) = g.entry() {
            if !graph.vnodes().is_occupied(v_id.index()) {
                errors.push(format!(
                    "G-I4 violated at G-node {idx}: entry V-node {} is not occupied",
                    v_id.index()
                ));
                continue;
            }
            let v = graph.vnodes().get(v_id.index());

            let g_own = g.own().to_f64_approx();
            let v_int = v.intensity().to_f64_approx();
            #[allow(clippy::float_cmp)]
            if g_own != v_int && (g_own - v_int).abs() > 1e-9 {
                errors.push(format!(
                    "G-I4 violated at G-node {idx}: g.own={g_own}, entry.intensity={v_int}"
                ));
            }

            match &v.kind() {
                VKind::Entry { gnode, .. } => {
                    let g_id = GNodeId::from_index(idx);
                    if *gnode != g_id {
                        errors.push(format!(
                            "G-I4 violated at G-node {idx}: entry's gnode={gnode:?}, expected {g_id:?}"
                        ));
                    }
                }
                VKind::Structural { .. } => {
                    errors.push(format!(
                        "G-I4 violated at G-node {idx}: entry is a structural V-node, not an entry"
                    ));
                }
            }
        }
    }
}

#[allow(clippy::float_cmp)]
/// G-I5: every occupied G-node must have exactly one V-Entry, and the total
/// count of occupied G-nodes must equal the total count of `VKind::Entry` nodes.
/// This asserts the 1-to-1 bijection between G-nodes and V-Entry nodes.
fn check_g_i5_entry_bijection<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    let mut g_without_entry = Vec::new();
    let mut g_count = 0usize;
    for (idx, g) in graph.core.gtree.nodes.iter_occupied() {
        g_count += 1;
        if g.entry().is_none() {
            g_without_entry.push(idx);
        }
    }

    for idx in &g_without_entry {
        errors.push(format!(
            "G-I5 violated at G-node {idx}: no V-Entry — every G-node must have exactly one"
        ));
    }

    let v_entry_count = graph
        .vnodes()
        .iter_occupied()
        .filter(|(_, v)| matches!(v.kind(), VKind::Entry { .. }))
        .count();

    if g_count != v_entry_count {
        errors.push(format!(
            "G-I5 violated: {g_count} occupied G-nodes but {v_entry_count} V-Entry nodes (must be equal)"
        ));
    }
}

fn check_v_i1_structural_sum<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        if let VKind::Structural { children, .. } = &v.kind() {
            let mut sum = 0.0_f64;
            for i in 0..children.len() {
                let (child_id, cached_int) = children.get(i);

                if !graph.vnodes().is_occupied(child_id.index()) {
                    errors.push(format!(
                        "V-I1 violated at V-node {idx}: child {} is not occupied",
                        child_id.index()
                    ));
                    continue;
                }
                let actual_int = graph.vnodes().get(child_id.index()).intensity();
                if (cached_int.to_f64_approx() - actual_int.to_f64_approx()).abs() > 1e-9 {
                    errors.push(format!(
                        "V-I1 cached intensity mismatch at V-node {idx}, child {}: \
                         cached={}, actual={}",
                        child_id.index(),
                        cached_int.to_f64_approx(),
                        actual_int.to_f64_approx()
                    ));
                }
                sum += cached_int.to_f64_approx();
            }

            let node_int = v.intensity().to_f64_approx();
            if (node_int - sum).abs() > 1e-9 {
                errors.push(format!(
                    "V-I1 violated at V-node {idx}: intensity={node_int}, sum of children={sum}"
                ));
            }
        }
    }
}

fn check_v_i2_branching_factor<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        if let VKind::Structural { children, .. } = &v.kind() {
            let len = children.len();
            if len != 2 && len != 3 {
                errors.push(format!(
                    "V-I2 violated at V-node {idx}: {len} children (must be 2 or 3)"
                ));
            }
        }
    }
}

fn check_v_i3_max_uncle<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        let v_id = VNodeId::from_index(idx);
        if graph.vnodes().is_violated(v_id) {
            let int = v.intensity().to_f64_approx();
            let uncle = graph
                .vnodes()
                .max_uncle_intensity(v_id)
                .map_or(f64::NAN, Inspectable::to_f64_approx);
            errors.push(format!(
                "V-I3 violated at V-node {idx}: intensity={int}, max_uncle={uncle}"
            ));
        }
    }
}

fn check_v_i5_entry_leaf<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        if let VKind::Structural { children, .. } = &v.kind() {
            for i in 0..children.len() {
                let (child_id, _) = children.get(i);
                if !graph.vnodes().is_occupied(child_id.index()) {
                    errors.push(format!(
                        "V-I5 violated at V-node {idx}: child {} is not occupied",
                        child_id.index()
                    ));
                }
            }
        }
    }
}

fn check_v_i6_exposed_flag<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        if let VKind::Entry {
            gnode, is_exposed, ..
        } = &v.kind()
        {
            if !graph.core.gtree.nodes.is_occupied(gnode.index()) {
                errors.push(format!(
                    "V-I6 violated at V-node {idx}: backing G-node {} is not occupied",
                    gnode.index()
                ));
                continue;
            }
            let g = graph.core.gtree.nodes.get(gnode.index());
            let expected = g.uncovered_range().is_some();
            if *is_exposed != expected {
                errors.push(format!(
                    "V-I6 violated at V-node {idx}: is_exposed={is_exposed}, \
                     expected={expected} (state={:?})",
                    g.state()
                ));
            }
        }
    }
}

fn check_v_i6b_evictable_flag<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        if let VKind::Entry {
            gnode,
            is_evictable,
            is_exposed,
            ..
        } = &v.kind()
        {
            if !graph.core.gtree.nodes.is_occupied(gnode.index()) {
                continue;
            }
            let g = graph.core.gtree.nodes.get(gnode.index());
            let expected = g.is_terminal();
            if *is_evictable != expected {
                errors.push(format!(
                    "V-I6b violated at V-node {idx}: is_evictable={is_evictable}, \
                     expected={expected} (state={:?})",
                    g.state()
                ));
            }

            if *is_evictable && !*is_exposed {
                errors.push(format!(
                    "V-I6b violated at V-node {idx}: is_evictable=true but is_exposed=false"
                ));
            }
        }
    }
}

fn check_v_i7_structural_flag<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        if let VKind::Structural {
            children,
            has_evictable,
        } = &v.kind()
        {
            let expected = (0..children.len()).any(|i| {
                let (child_id, _) = children.get(i);
                if !graph.vnodes().is_occupied(child_id.index()) {
                    return false;
                }
                let child = graph.vnodes().get(child_id.index());
                match &child.kind() {
                    VKind::Entry { is_evictable, .. } => *is_evictable,
                    VKind::Structural { has_evictable, .. } => *has_evictable,
                }
            });
            if *has_evictable != expected {
                errors.push(format!(
                    "V-I7 violated at V-node {idx}: has_evictable={has_evictable}, \
                     expected={expected}"
                ));
            }
        }
    }
}

#[allow(clippy::float_cmp)]
fn check_clean_accounting<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    let mut total_v = 0.0_f64;
    for (_, v) in graph.vnodes().iter_occupied() {
        if matches!(v.kind(), VKind::Entry { .. }) {
            total_v += v.intensity().to_f64_approx();
        }
    }
    let g_root_sum = graph
        .core
        .gtree
        .nodes
        .get(graph.core.gtree.nodes.root.index())
        .sum()
        .to_f64_approx();
    if total_v != g_root_sum && (total_v - g_root_sum).abs() > 1e-9 {
        errors.push(format!(
            "Clean accounting violated: V-entry sum={total_v}, G-root sum={g_root_sum}"
        ));
    }
}

fn check_parent_link_consistency<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    check_g_parent_links(graph, errors);
    check_v_parent_links(graph, errors);
}

/// Verifies that every G-node's left and right children point back to it as
/// their parent.
fn check_g_parent_links<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, g) in graph.core.gtree.nodes.iter_occupied() {
        let g_id = GNodeId::from_index(idx);
        if let Some(left) = g.left() {
            if graph.core.gtree.nodes.is_occupied(left.index()) {
                let child_parent = graph.core.gtree.nodes.get(left.index()).parent();
                if child_parent != Some(g_id) {
                    errors.push(format!(
                        "G-parent link: G-node {idx}'s left child {}'s parent is {:?}, expected {g_id:?}",
                        left.index(),
                        child_parent
                    ));
                }
            }
        }
        if let Some(right) = g.right() {
            if graph.core.gtree.nodes.is_occupied(right.index()) {
                let child_parent = graph.core.gtree.nodes.get(right.index()).parent();
                if child_parent != Some(g_id) {
                    errors.push(format!(
                        "G-parent link: G-node {idx}'s right child {}'s parent is {:?}, expected {g_id:?}",
                        right.index(),
                        child_parent
                    ));
                }
            }
        }
    }
}

/// Verifies V-tree parent–child consistency: every structural child points back
/// to its parent, and every node's claimed parent lists it as a child.
fn check_v_parent_links<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    for (idx, v) in graph.vnodes().iter_occupied() {
        let v_id = VNodeId::from_index(idx);
        if let VKind::Structural { children, .. } = &v.kind() {
            for i in 0..children.len() {
                let (child_id, _) = children.get(i);
                if graph.vnodes().is_occupied(child_id.index()) {
                    let child_parent = graph.vnodes().get(child_id.index()).parent();
                    if child_parent != Some(v_id) {
                        errors.push(format!(
                            "V-parent link: V-node {idx}'s child {}'s parent is {:?}, expected {v_id:?}",
                            child_id.index(),
                            child_parent
                        ));
                    }
                }
            }
        }

        if let Some(p_id) = v.parent() {
            if graph.vnodes().is_occupied(p_id.index()) {
                let parent = graph.vnodes().get(p_id.index());
                if let VKind::Structural { children, .. } = &parent.kind() {
                    if children.find_index(v_id).is_none() {
                        errors.push(format!(
                            "V-parent link: V-node {idx} has parent {}, but parent does not list it as a child",
                            p_id.index()
                        ));
                    }
                } else {
                    errors.push(format!(
                        "V-parent link: V-node {idx} has parent {}, but parent is not structural",
                        p_id.index()
                    ));
                }
            }
        }
    }
}

fn check_v_root_consistency<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    if let Some(root_id) = graph.v_root() {
        if !graph.vnodes().is_occupied(root_id.index()) {
            errors.push(format!(
                "V-root consistency: v_root {} is not occupied",
                root_id.index()
            ));
            return;
        }
        let root = graph.vnodes().get(root_id.index());
        if root.parent().is_some() {
            errors.push(format!(
                "V-root consistency: v_root {} has parent {:?}, expected None",
                root_id.index(),
                root.parent()
            ));
        }
    }
}

fn check_node_count_consistency<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    let actual = graph.core.gtree.nodes.count();
    let expected = graph.core.gtree.nodes.node_count;
    if actual != expected {
        errors.push(format!(
            "Node count: graph.core.gtree.nodes.node_count={expected}, arena count={actual}"
        ));
    }
}

fn check_terminal_count_consistency<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    #[allow(clippy::cast_possible_truncation)]
    let actual = graph
        .core
        .gtree
        .nodes
        .iter_occupied()
        .filter(|(_, g)| g.is_terminal())
        .count() as u32;
    let expected = graph.core.gtree.nodes.terminal_count;
    if actual != expected {
        errors.push(format!(
            "Terminal count: graph.core.gtree.nodes.terminal_count={expected}, arena walk={actual}"
        ));
    }
}

fn check_hard_budget<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    if let Some(budget) = graph.config().structural.budget {
        if graph.core.gtree.nodes.node_count as usize > budget {
            errors.push(format!(
                "Hard budget violated (ADR-M-018): node_count ({}) > budget ({})",
                graph.core.gtree.nodes.node_count, budget
            ));
        }
    }
}

fn check_depth_gate_invariants<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    let d_create = graph.depth_create();
    let d_evict = graph.depth_evict();
    let buffer = graph.core.gtree.depth_buffer;

    if d_create >= d_evict {
        errors.push(format!(
            "D-I3: live_depth_create ({d_create}) must be < live_depth_evict ({d_evict})"
        ));
    }

    if d_evict < buffer + 1 {
        errors.push(format!(
            "D-I3 floor: live_depth_evict ({d_evict}) < depth_buffer ({buffer}) + 1"
        ));
    }

    if d_evict >= buffer && d_create != d_evict - buffer {
        errors.push(format!(
            "D-I3 buffer: live_depth_create ({d_create}) != live_depth_evict ({d_evict}) - depth_buffer ({buffer})"
        ));
    }
}

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Config, StructuralConfig};
    use crate::tree::handle::VNodeId;
    use crate::tree::vtree::vnode::{VKind, VNode};

    type G = GvGraph<u8, u32, 8>;

    fn make_config(budget: Option<usize>) -> Config<u32> {
        Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 3,
                depth_evict: 5,
                budget,
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        }
    }

    #[test]
    fn check_all_invariants_is_empty_for_fresh_graph() {
        let g: G = GvGraph::new(make_config(None));
        let errors = check_all_invariants(&g);
        assert!(errors.is_empty());
    }

    #[test]
    fn check_node_count_consistency_reports_mismatch() {
        let mut g: G = GvGraph::new(make_config(None));
        g.core.gtree.nodes.node_count += 1;

        let mut errors = Vec::new();
        check_node_count_consistency(&g, &mut errors);

        assert!(
            errors.iter().any(|e| e.contains("Node count:")),
            "expected node count mismatch error, got: {errors:?}"
        );
    }

    #[test]
    fn check_terminal_count_consistency_reports_mismatch() {
        let mut g: G = GvGraph::new(make_config(None));
        g.core.gtree.nodes.terminal_count += 1;

        let mut errors = Vec::new();
        check_terminal_count_consistency(&g, &mut errors);

        assert!(
            errors.iter().any(|e| e.contains("Terminal count:")),
            "expected terminal count mismatch error, got: {errors:?}"
        );
    }

    #[test]
    fn check_v_root_consistency_reports_parented_root() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.v_root().expect("fresh graph must have a v_root");
        g.core.vtree.nodes.get_mut(root.index()).set_parent(root);

        let mut errors = Vec::new();
        check_v_root_consistency(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("v_root") && e.contains("expected None")),
            "expected v_root parent consistency error, got: {errors:?}"
        );
    }

    #[test]
    fn check_clean_accounting_reports_entry_sum_mismatch() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.v_root().expect("fresh graph must have a v_root");
        g.core.vtree.nodes.get_mut(root.index()).set_intensity(1);

        let mut errors = Vec::new();
        check_clean_accounting(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("Clean accounting violated")),
            "expected clean-accounting error, got: {errors:?}"
        );
    }

    #[test]
    fn check_depth_gate_invariants_reports_invalid_ordering() {
        let mut g: G = GvGraph::new(make_config(None));
        g.core.gtree.live_depth_create = g.core.gtree.live_depth_evict;

        let mut errors = Vec::new();
        check_depth_gate_invariants(&g, &mut errors);

        assert!(
            errors.iter().any(|e| e.contains("D-I3:")),
            "expected D-I3 ordering error, got: {errors:?}"
        );
    }

    #[test]
    fn check_hard_budget_reports_exceeded_budget() {
        let mut g: G = GvGraph::new(make_config(Some(30)));
        g.core.gtree.nodes.node_count = 31;

        let mut errors = Vec::new();
        check_hard_budget(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("Hard budget violated") && e.contains("node_count")),
            "expected hard-budget error, got: {errors:?}"
        );
    }

    #[test]
    fn check_all_invariants_reports_multiple_corruptions() {
        let mut g: G = GvGraph::new(make_config(Some(30)));
        g.observe(64u8, 3u32); // create structural V-root and multiple G-nodes

        // Break accounting counters.
        g.core.gtree.nodes.node_count += 2;
        g.core.gtree.nodes.terminal_count += 1;

        // Break depth-gate relation.
        g.core.gtree.live_depth_create = g.core.gtree.live_depth_evict;

        // Break V-root parent consistency.
        let v_root = g.v_root().expect("v_root must exist");
        g.core
            .vtree
            .nodes
            .get_mut(v_root.index())
            .set_parent(v_root);

        // Break clean accounting and G-I4/V-I6b-style consistency by mutating
        // one entry's intensity and flags away from its backing G-node state.
        let root_entry = g
            .core
            .gtree
            .nodes
            .get(g.core.gtree.nodes.root.index())
            .entry()
            .expect("root must have entry");
        {
            let v = g.core.vtree.nodes.get_mut(root_entry.index());
            v.set_intensity(999u32);
            if let VKind::Entry {
                is_exposed,
                is_evictable,
                ..
            } = v.kind_mut()
            {
                *is_exposed = false;
                *is_evictable = true;
            }
        }

        // Break structural evictable aggregation flag on the V-root.
        if let VKind::Structural { has_evictable, .. } =
            g.core.vtree.nodes.get_mut(v_root.index()).kind_mut()
        {
            *has_evictable = false;
        }

        let errors = check_all_invariants(&g);
        assert!(!errors.is_empty(), "expected multiple invariant violations");
        assert!(errors.iter().any(|e| e.contains("Node count:")));
        assert!(errors.iter().any(|e| e.contains("Terminal count:")));
        assert!(errors.iter().any(|e| e.contains("D-I3:")));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("v_root") || e.contains("V-root consistency"))
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("Clean accounting violated"))
        );
    }

    #[test]
    fn check_g_i4_entry_consistency_reports_unoccupied_entry_id() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.core.gtree.nodes.root;
        g.core
            .gtree
            .nodes
            .get_mut(root.index())
            .assign_entry(VNodeId::from_index(9_999));

        let mut errors = Vec::new();
        check_g_i4_entry_consistency(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("entry V-node") && e.contains("not occupied"))
        );
    }

    #[test]
    fn check_g_i1_summation_reports_mismatch() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.core.gtree.nodes.root;
        g.core.gtree.nodes.get_mut(root.index()).set_sum(123u32);

        let mut errors = Vec::new();
        check_g_i1_summation(&g, &mut errors);

        assert!(errors.iter().any(|e| e.contains("G-I1 violated")));
    }

    #[test]
    fn check_g_i4_entry_consistency_reports_gnode_mismatch_and_structural_entry() {
        let mut g: G = GvGraph::new(make_config(None));
        g.observe(64u8, 3u32);

        let root_gid = g.core.gtree.nodes.root;
        let (child_gid, child_entry) = {
            let root = g.core.gtree.nodes.get(root_gid.index());
            let child_gid = root.left().expect("left child should exist after split");
            let child_entry = g
                .core
                .gtree
                .nodes
                .get(child_gid.index())
                .entry()
                .expect("child should have entry");
            (child_gid, child_entry)
        };

        if let VKind::Entry { gnode, .. } =
            g.core.vtree.nodes.get_mut(child_entry.index()).kind_mut()
        {
            *gnode = GNodeId::from_index(root_gid.index());
        }

        let v_root = g.v_root().expect("v_root must exist");
        g.core
            .gtree
            .nodes
            .get_mut(root_gid.index())
            .assign_entry(v_root);

        let mut errors = Vec::new();
        check_g_i4_entry_consistency(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("entry's gnode") && e.contains(&child_gid.index().to_string()))
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("entry is a structural V-node"))
        );
    }

    #[test]
    fn check_g_i5_entry_bijection_reports_missing_entry_and_count_mismatch() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.core.gtree.nodes.root;
        g.core.gtree.nodes.get_mut(root.index()).clear_entry();
        let extra = VNode::new_entry(0u32, None, root, true, true);
        let _ = g.core.vtree.nodes.alloc(extra);

        let mut errors = Vec::new();
        check_g_i5_entry_bijection(&g, &mut errors);

        assert!(errors.iter().any(|e| e.contains("no V-Entry")));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("occupied G-nodes") && e.contains("V-Entry nodes"))
        );
    }

    #[test]
    fn check_v_i1_and_v_i5_report_unoccupied_child() {
        let mut g: G = GvGraph::new(make_config(None));
        g.observe(64u8, 3u32);

        let v_root = g.v_root().expect("v_root must exist after split");
        let child = match g.core.vtree.nodes.get(v_root.index()).kind() {
            VKind::Structural { children, .. } => children.get(0).0,
            VKind::Entry { .. } => panic!("expected structural v_root"),
        };
        g.core.vtree.nodes.dealloc(child.index());

        let mut errors = Vec::new();
        check_v_i1_structural_sum(&g, &mut errors);
        check_v_i5_entry_leaf(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("V-I1 violated") && e.contains("not occupied"))
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("V-I5 violated") && e.contains("not occupied"))
        );
    }

    #[test]
    fn check_v_i6_reports_unoccupied_backing_gnode() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.v_root().expect("fresh graph must have v_root");
        if let VKind::Entry { gnode, .. } = g.core.vtree.nodes.get_mut(root.index()).kind_mut() {
            *gnode = GNodeId::from_index(999);
        }

        let mut errors = Vec::new();
        check_v_i6_exposed_flag(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("backing G-node") && e.contains("not occupied"))
        );
    }

    #[test]
    fn check_v_parent_links_reports_non_structural_parent() {
        let mut g: G = GvGraph::new(make_config(None));
        g.observe(64u8, 3u32); // ensure multiple occupied vnodes

        let root_entry = g
            .core
            .gtree
            .nodes
            .get(g.core.gtree.nodes.root.index())
            .entry()
            .expect("root must have entry");
        let other_entry = g
            .vnodes()
            .iter_occupied()
            .map(|(idx, _)| VNodeId::from_index(idx))
            .find(|id| *id != root_entry)
            .expect("expected a second vnode after split");

        g.core
            .vtree
            .nodes
            .get_mut(other_entry.index())
            .set_parent(root_entry);

        let mut errors = Vec::new();
        check_v_parent_links(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("parent is not structural"))
        );
    }

    #[test]
    fn check_v_i6b_reports_evictable_without_exposed() {
        let mut g: G = GvGraph::new(make_config(None));
        let root = g.v_root().expect("fresh graph must have v_root");
        if let VKind::Entry {
            is_evictable,
            is_exposed,
            ..
        } = g.core.vtree.nodes.get_mut(root.index()).kind_mut()
        {
            *is_evictable = true;
            *is_exposed = false;
        }

        let mut errors = Vec::new();
        check_v_i6b_evictable_flag(&g, &mut errors);

        assert!(
            errors
                .iter()
                .any(|e| e.contains("is_evictable=true") && e.contains("is_exposed=false"))
        );
    }

    #[test]
    fn check_v_i7_reports_has_evictable_mismatch() {
        let mut g: G = GvGraph::new(make_config(None));
        g.observe(64u8, 3u32); // produce structural v-root
        let v_root = g.v_root().expect("v_root must exist");
        if let VKind::Structural { has_evictable, .. } =
            g.core.vtree.nodes.get_mut(v_root.index()).kind_mut()
        {
            *has_evictable = false;
        }

        let mut errors = Vec::new();
        check_v_i7_structural_flag(&g, &mut errors);

        assert!(errors.iter().any(|e| e.contains("V-I7 violated")));
    }

    #[test]
    fn check_depth_gate_invariants_reports_floor_and_buffer_errors() {
        let mut g: G = GvGraph::new(make_config(None));
        let buffer = g.core.gtree.depth_buffer;
        g.core.gtree.live_depth_evict = buffer;
        g.core.gtree.live_depth_create = 1;

        let mut errors = Vec::new();
        check_depth_gate_invariants(&g, &mut errors);

        assert!(errors.iter().any(|e| e.contains("D-I3 floor")));
        assert!(errors.iter().any(|e| e.contains("D-I3 buffer")));
    }

    #[test]
    fn grouped_invariant_wrappers_run_on_fresh_graph() {
        let g: G = GvGraph::new(make_config(None));
        let mut errors = Vec::new();

        check_g_tree_invariants(&g, &mut errors);
        check_v_tree_invariants(&g, &mut errors);
        check_accounting_invariants(&g, &mut errors);

        assert!(
            errors.is_empty(),
            "fresh graph should satisfy grouped invariants: {errors:?}"
        );
    }

    #[cfg(feature = "dynamic-contour-tracking")]
    #[test]
    fn plateau_only_helpers_run_without_errors_on_fresh_graph() {
        let g: G = GvGraph::new(make_config(None));
        let mut errors = Vec::new();

        check_plateau_only(&g, &mut errors);
        check_p_i3_only(&g, &mut errors);

        assert!(
            errors.is_empty(),
            "fresh graph should satisfy plateau-only checks: {errors:?}"
        );
    }
}
