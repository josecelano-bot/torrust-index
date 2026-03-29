use super::*;
use crate::graph::{Config, StructuralConfig};

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

#[test]
fn assert_invariants_succeeds_for_fresh_graph() {
    let g: G = GvGraph::new(make_config());
    assert_invariants(&g);
}

#[test]
fn check_v_root_consistency_reports_unoccupied_root() {
    let mut g: G = GvGraph::new(make_config());
    let root = g.v_root().expect("fresh graph must have v_root");
    let _removed = g.vtree.nodes.dealloc(root.index());

    let mut errors = Vec::new();
    check_v_root_consistency(&g, &mut errors);

    assert!(errors.iter().any(|e| e.contains("is not occupied")));
}

#[test]
fn parent_link_consistency_runs_with_stale_child_pointer() {
    let mut g: G = GvGraph::new(make_config());
    g.gtree.nodes.get_mut(g.gtree.root.index()).link_left(GNodeId::from_index(999));

    let mut errors = Vec::new();
    check_parent_link_consistency(&g, &mut errors);

    assert!(errors.iter().all(|e| !e.is_empty()));
}
