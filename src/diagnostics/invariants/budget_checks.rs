use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate, Inspectable};

/// Accounting: parent links, root, node/terminal counts, depth gates, budget.
pub(super) fn check_accounting_invariants<
    C: Coordinate,
    V: Accumulator + Inspectable,
    const N: u32,
>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    super::check_clean_accounting(graph, errors);
    super::check_parent_link_consistency(graph, errors);
    super::check_v_root_consistency(graph, errors);
    super::check_node_count_consistency(graph, errors);
    super::check_terminal_count_consistency(graph, errors);
    super::check_depth_gate_invariants(graph, errors);
    super::check_hard_budget(graph, errors);
}
