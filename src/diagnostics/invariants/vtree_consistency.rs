use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate, Inspectable};

/// V-I1..V-I7: intensity summation, branching, uncle, flag, and leaf invariants.
pub(super) fn check_v_tree_invariants<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    super::check_v_i1_structural_sum(graph, errors);
    super::check_v_i2_branching_factor(graph, errors);
    super::check_v_i3_max_uncle(graph, errors);
    super::check_v_i5_entry_leaf(graph, errors);
    super::check_v_i6_exposed_flag(graph, errors);
    super::check_v_i6b_evictable_flag(graph, errors);
    super::check_v_i7_structural_flag(graph, errors);
}
