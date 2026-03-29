use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate, Inspectable};

/// G-I1, G-I2, G-I4, G-I5: structural and summation invariants on the G-tree.
pub(super) fn check_g_tree_invariants<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    super::check_g_i1_summation(graph, errors);
    super::check_g_i2_variable_fanout(graph, errors);
    super::check_g_i4_entry_consistency(graph, errors);
    super::check_g_i5_entry_bijection(graph, errors);
}
