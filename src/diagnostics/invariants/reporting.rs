#[cfg(feature = "dynamic-contour-tracking")]
use crate::diagnostics::plateau_invariants::{
    check_p_i1_i_keys_are_contour_steps, check_p_i1_ii_tile_contiguity,
    check_p_i1_iii_run_contains_tile, check_p_i2_basis_minimality, check_p_i3_basis_disjointness,
    check_p_i4_thatch_one_hop, check_p_i5_thatch_depth, check_plateau_basis_consistency,
    check_plateau_btreemap_key_consistency, check_plateau_depth_consistency,
    check_plateau_sum_consistency,
};
use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate, Inspectable};

pub fn assert_invariants<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
) {
    let errors = check_all_invariants(graph);
    if !errors.is_empty() {
        let msg = errors.join("\n");
        panic!("Invariant violations ({} total):\n{msg}", errors.len());
    }
}

#[cfg(feature = "dynamic-contour-tracking")]
#[allow(dead_code)]
pub fn check_plateau_only<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    check_plateau_invariants(graph, errors);
}

#[cfg(feature = "dynamic-contour-tracking")]
#[allow(dead_code)]
pub fn check_p_i3_only<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    check_p_i3_basis_disjointness(graph, errors);
}

#[allow(dead_code)]
pub fn check_all_invariants<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    super::check_g_tree_invariants(graph, &mut errors);
    super::check_v_tree_invariants(graph, &mut errors);
    super::check_accounting_invariants(graph, &mut errors);
    #[cfg(feature = "dynamic-contour-tracking")]
    check_plateau_invariants(graph, &mut errors);

    errors
}

/// P-I1..P-I5: plateau key, basis, sum, depth and thatch invariants.
/// Only compiled when the `dynamic-contour-tracking` feature is enabled.
#[cfg(feature = "dynamic-contour-tracking")]
fn check_plateau_invariants<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    errors: &mut Vec<String>,
) {
    check_plateau_btreemap_key_consistency(graph, errors);
    check_plateau_basis_consistency(graph, errors);
    check_plateau_sum_consistency(graph, errors);
    check_plateau_depth_consistency(graph, errors);
    check_p_i1_i_keys_are_contour_steps(graph, errors);
    check_p_i1_ii_tile_contiguity(graph, errors);
    check_p_i1_iii_run_contains_tile(graph, errors);
    check_p_i2_basis_minimality(graph, errors);
    check_p_i3_basis_disjointness(graph, errors);
    check_p_i4_thatch_one_hop(graph, errors);
    check_p_i5_thatch_depth(graph, errors);
}
