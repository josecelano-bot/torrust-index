use crate::graph::GvGraph;
use crate::handle::VNodeId;
use crate::nodes::gnode::GState;
use crate::nodes::vnode::VKind;
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::gtree::GTree;

const fn state_label(s: GState) -> &'static str {
    match s {
        GState::Terminal => "T",
        GState::SemiInternal => "S",
        GState::Internal => "I",
    }
}

/// Render the **G-Tree** (binary range-split tree) as a Graphviz DOT string.
///
/// Each node shows:
/// - node id, coordinate range `[lo, hi)`
/// - geometric depth (`d`), node state (`T`=Terminal, `I`=Internal,
///   `S`=`SemiInternal`)
/// - accumulated `own` value and subtree `sum`
/// - whether a V-Tree Entry is currently attached
///
/// Nodes are colour-coded by state:
/// - green  — Terminal with a V-tree entry
/// - grey   — Terminal without a V-tree entry (orphaned leaf)
/// - yellow — Internal (both children present)
/// - orange — `SemiInternal` (one child present)///
/// # Panics
///
/// Panics if a terminal G-node is marked as having a V-tree entry but the
/// entry handle is `None` (internal invariant violation).
pub fn dump_gtree_dot<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    label: &str,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    writeln!(out, "digraph gtree {{").expect("writing DOT output failed");
    writeln!(out, "  label={label:?};").expect("writing DOT output failed");
    writeln!(out, "  rankdir=TB;").expect("writing DOT output failed");
    writeln!(
        out,
        "  node [shape=box, fontname=\"Courier New\", fontsize=11];"
    )
    .expect("writing DOT output failed");
    writeln!(out).expect("writing DOT output failed");

    // BFS from the root so the node ordering in the file is breadth-first.
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(graph.core.gtree.nodes.root);

    while let Some(gid) = queue.pop_front() {
        let g = graph.core.gtree.nodes.get(gid.index());
        let idx = gid.index();
        let depth = GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi());
        let state = state_label(g.state());
        let has_entry = g.entry().is_some();

        let fillcolor = match g.state() {
            GState::Terminal if has_entry => "#c8e6c9", // green
            GState::Terminal => "#e0e0e0",              // grey
            GState::Internal => "#fff9c4",              // yellow
            GState::SemiInternal => "#ffe0b2",          // orange
        };

        let entry_note = if has_entry {
            format!(
                "VEntry({})",
                g.entry().expect("writing DOT output failed").index()
            )
        } else {
            "no VEntry".to_string()
        };

        writeln!(
            out,
            "  G{idx} [label=\"G{idx}  [{:.0}, {:.0})\\nd={depth}  {state}  {entry_note}\\nown={:.0}  sum={:.0}\", \
             style=filled, fillcolor=\"{fillcolor}\"];",
            g.lo().to_f64(),
            g.hi().to_f64(),
            g.own().to_f64_approx(),
            g.sum().to_f64_approx(),
        )
        .expect("writing DOT output failed");

        if let Some(left) = g.left() {
            queue.push_back(left);
        }
        if let Some(right) = g.right() {
            queue.push_back(right);
        }
    }

    writeln!(out).expect("writing DOT output failed");

    // Edges (separate pass so all nodes are declared before edges).
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(graph.core.gtree.nodes.root);
    while let Some(gid) = queue.pop_front() {
        let g = graph.core.gtree.nodes.get(gid.index());
        let idx = gid.index();
        if let Some(left) = g.left() {
            writeln!(out, "  G{idx} -> G{} [label=\"L\"];", left.index())
                .expect("writing DOT output failed");
            queue.push_back(left);
        }
        if let Some(right) = g.right() {
            writeln!(out, "  G{idx} -> G{} [label=\"R\"];", right.index())
                .expect("writing DOT output failed");
            queue.push_back(right);
        }
    }

    writeln!(out, "}}").expect("writing DOT output failed");
    out
}

/// Render the **V-Tree** (virtual intensity tree) as a Graphviz DOT string.
///
/// Two kinds of V-nodes are distinguished:
/// - **Entry** (blue box) — leaf in the V-Tree, backed by a real G-node. Shows
///   the linked G-node id, its coordinate range, geometric depth, state,
///   the accumulated intensity, and the `exposed` / `evictable` flags.
/// - **Structural** (purple ellipse) — internal routing node that aggregates
///   child intensities.  Shows the aggregated intensity and the
///   `has_evictable` flag.
#[allow(dead_code)]
pub fn dump_vtree_dot<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    graph: &GvGraph<C, V, N>,
    label: &str,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    let Some(v_root) = graph.v_root() else {
        writeln!(out, "digraph vtree {{").expect("writing DOT output failed");
        writeln!(out, "  label={label:?};").expect("writing DOT output failed");
        writeln!(
            out,
            "  empty [label=\"(empty — no observations yet)\", shape=plaintext];"
        )
        .expect("writing DOT output failed");
        writeln!(out, "}}").expect("writing DOT output failed");
        return out;
    };

    writeln!(out, "digraph vtree {{").expect("writing DOT output failed");
    writeln!(out, "  label={label:?};").expect("writing DOT output failed");
    writeln!(out, "  rankdir=TB;").expect("writing DOT output failed");
    writeln!(out, "  node [fontname=\"Courier New\", fontsize=11];")
        .expect("writing DOT output failed");
    writeln!(out).expect("writing DOT output failed");

    // Declare nodes (BFS).
    let mut queue: std::collections::VecDeque<VNodeId> = std::collections::VecDeque::new();
    queue.push_back(v_root);

    while let Some(vid) = queue.pop_front() {
        let vnode = graph.vnodes().get(vid.index());
        let idx = vid.index();

        match &vnode.kind() {
            VKind::Entry {
                gnode,
                is_exposed,
                is_evictable,
            } => {
                let g = graph.core.gtree.nodes.get(gnode.index());
                let g_depth = GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi());
                let g_state = state_label(g.state());
                writeln!(
                    out,
                    "  V{idx} [shape=box, style=filled, fillcolor=\"#bbdefb\", \
                     label=\"V{idx} Entry\\nG{}  [{:.0},{:.0}) d={g_depth} {g_state}\\n\
                     intensity={:.0}  exposed={is_exposed}  evict={is_evictable}\"];",
                    gnode.index(),
                    g.lo().to_f64(),
                    g.hi().to_f64(),
                    vnode.intensity().to_f64_approx(),
                )
                .expect("writing DOT output failed");
            }
            VKind::Structural {
                children,
                has_evictable,
            } => {
                writeln!(
                    out,
                    "  V{idx} [shape=ellipse, style=filled, fillcolor=\"#e1bee7\", \
                     label=\"V{idx} Struct\\nintensity={:.0}  has_evict={has_evictable}\"];",
                    vnode.intensity().to_f64_approx(),
                )
                .expect("writing DOT output failed");
                for i in 0..children.len() {
                    let (child_id, _) = children.get(i);
                    queue.push_back(child_id);
                }
            }
        }
    }

    writeln!(out).expect("writing DOT output failed");

    // Edges (separate BFS pass).
    let mut queue: std::collections::VecDeque<VNodeId> = std::collections::VecDeque::new();
    queue.push_back(v_root);

    while let Some(vid) = queue.pop_front() {
        let vnode = graph.vnodes().get(vid.index());
        let idx = vid.index();

        if let VKind::Structural { children, .. } = &vnode.kind() {
            for i in 0..children.len() {
                let (child_id, child_intensity) = children.get(i);
                writeln!(
                    out,
                    "  V{idx} -> V{} [label=\"i={:.0}\"];",
                    child_id.index(),
                    child_intensity.to_f64_approx(),
                )
                .expect("writing DOT output failed");
                queue.push_back(child_id);
            }
        }
    }

    writeln!(out, "}}").expect("writing DOT output failed");
    out
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn dump_gtree_dot_on_fresh_graph_produces_valid_dot() {
        let g: G = GvGraph::new(make_config());
        let dot = super::dump_gtree_dot(&g, "fresh");
        assert!(dot.contains("digraph gtree"), "missing header: {dot}");
        assert!(dot.contains("label=\"fresh\""), "missing label: {dot}");
        // Fresh graph has a single Terminal node with a VEntry → green fill.
        assert!(
            dot.contains("#c8e6c9"),
            "expected green for terminal-with-entry: {dot}"
        );
    }

    #[test]
    fn dump_gtree_dot_after_observation_includes_entry_node() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 5u32);
        let dot = super::dump_gtree_dot(&g, "obs1");
        // After one observation the root Terminal has a VEntry → green fill.
        assert!(
            dot.contains("#c8e6c9"),
            "expected green for terminal-with-entry: {dot}"
        );
    }

    #[test]
    fn dump_gtree_dot_after_split_includes_internal_nodes() {
        let mut g: G = GvGraph::new(make_config());
        // Trigger splits by repeating an observation (split_threshold = 2).
        for _ in 0..4 {
            g.observe(64u8, 5u32);
        }
        let dot = super::dump_gtree_dot(&g, "after_split");
        // After splits there should be Internal nodes (yellow) or SemiInternal (orange).
        let has_internal = dot.contains("#fff9c4") || dot.contains("#ffe0b2");
        assert!(
            has_internal,
            "expected yellow or orange fill after splits: {dot}"
        );
    }

    #[test]
    fn dump_vtree_dot_on_fresh_graph_produces_empty_body() {
        let g: G = GvGraph::new(make_config());
        // No observations → no v_root → empty-graph body.
        let dot = super::dump_vtree_dot(&g, "empty");
        assert!(dot.contains("digraph vtree"), "missing header: {dot}");
        assert!(dot.contains("empty"), "expected empty placeholder: {dot}");
    }

    #[test]
    fn dump_vtree_dot_after_observation_includes_entry_node() {
        let mut g: G = GvGraph::new(make_config());
        g.observe(64u8, 5u32);
        let dot = super::dump_vtree_dot(&g, "obs1");
        assert!(dot.contains("digraph vtree"), "missing header: {dot}");
        // Should contain at least one VEntry node (blue box).
        assert!(dot.contains("#bbdefb"), "expected blue entry node: {dot}");
    }

    #[test]
    fn dump_vtree_dot_after_split_includes_structural_node() {
        let mut g: G = GvGraph::new(make_config());
        for _ in 0..4 {
            g.observe(64u8, 5u32);
        }
        let dot = super::dump_vtree_dot(&g, "after_split");
        // Structural nodes are rendered as purple ellipses.
        assert!(
            dot.contains("#e1bee7"),
            "expected purple structural node: {dot}"
        );
        // Edges between structural and entry nodes should exist.
        assert!(dot.contains("->"), "expected edge arrows: {dot}");
    }
}
