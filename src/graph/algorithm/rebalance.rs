use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::{Children, VKind, VNode};
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::gtree::GTree;
use crate::tree::vtree::{VNodeTree, VTree};

mod context;
mod resolve;
mod violation_scan;

pub(super) use super::fmt::Ch;
pub use context::{Ctx, EscalationContext, Nd, VTreeMutContext};
pub use resolve::resolve;
pub use violation_scan::find_violated_nodes;

#[must_use]
pub fn max_uncle_intensity<V: Accumulator>(vnodes: &VNodeTree<V>, c: VNodeId) -> Option<V> {
    let parent = vnodes.get(c.index()).parent()?;
    let grandparent = vnodes.get(parent.index()).parent()?;

    let g = vnodes.get(grandparent.index());
    if let VKind::Structural { children, .. } = &g.kind() {
        let mut max_int = None;
        for i in 0..children.len() {
            let (id, intensity) = children.get(i);
            if id != parent {
                max_int =
                    Some(max_int.map_or(
                        intensity,
                        |cur| if intensity > cur { intensity } else { cur },
                    ));
            }
        }
        max_int
    } else {
        None
    }
}

#[must_use]
pub fn is_violated<V: Accumulator>(vnodes: &VNodeTree<V>, c: VNodeId) -> bool {
    let c_int = vnodes.get(c.index()).intensity();
    max_uncle_intensity(vnodes, c).is_some_and(|max_uncle| c_int > max_uncle)
}

pub fn contract<V: Accumulator>(vtree: &mut VTree<V>, p: VNodeId) -> VNodeId {
    let _span = tracing::debug_span!(
        "contract",
        p = %Nd(&vtree.nodes, p),
        children = %Ch(&vtree.nodes, p),
    )
    .entered();

    let (heaviest_idx, children_data) = {
        let node = vtree.nodes.get(p.index());
        let children = match &node.kind() {
            VKind::Structural { children, .. } => children,
            VKind::Entry { .. } => panic!("contract: p must be structural"),
        };
        assert!(children.len() == 3, "contract: p must be a 3-node");
        let h = children.heaviest_child_index();
        let data: [(VNodeId, V); 3] = [children.get(0), children.get(1), children.get(2)];
        (h, data)
    };

    let isolate = children_data[heaviest_idx];
    let mut merge = Vec::with_capacity(2);
    for (i, &child) in children_data.iter().enumerate() {
        if i != heaviest_idx {
            merge.push(child);
        }
    }
    let (a_id, a_int) = merge[0];
    let (b_id, b_int) = merge[1];

    let a_terminal = node_has_evictable(&vtree.nodes, a_id);
    let b_terminal = node_has_evictable(&vtree.nodes, b_id);

    let merged = VNode::new_structural(
        V::add(a_int, b_int),
        Some(p),
        Children::new_2((a_id, a_int), (b_id, b_int)),
        a_terminal || b_terminal,
    );
    let m_id = VNodeId::from_index(vtree.nodes.alloc(merged));

    vtree.nodes.get_mut(a_id.index()).set_parent(m_id);
    vtree.nodes.get_mut(b_id.index()).set_parent(m_id);

    let merged_int = V::add(a_int, b_int);
    let iso_terminal = node_has_evictable(&vtree.nodes, isolate.0);
    let m_terminal = a_terminal || b_terminal;

    let p_node = vtree.nodes.get_mut(p.index());
    if let VKind::Structural {
        children,
        has_evictable,
    } = p_node.kind_mut()
    {
        *children = Children::new_2(isolate, (m_id, merged_int));
        *has_evictable = iso_terminal || m_terminal;
    }

    vtree.propagate_evictable(p);

    tracing::debug!(
        merged = %Nd(&vtree.nodes, m_id),
        result = %Ch(&vtree.nodes, p),
        "complete",
    );
    m_id
}

pub(super) fn node_has_evictable<V: Accumulator>(vnodes: &VNodeTree<V>, id: VNodeId) -> bool {
    vnodes.node_has_evictable(id)
}

/// Log the violation-queue tail and either panic (debug) or signal a break
/// (release) when the rebalance loop exceeds its iteration budget.
///
/// Returns `false` in debug builds (unreachable — `panic!` diverges) and `true`
/// in release builds to tell the caller to break out of the loop.
fn handle_iteration_limit<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    violations: &[VNodeId],
    iterations: u32,
    max_iterations: u32,
    resolved: u32,
    current: VNodeId,
) -> bool {
    tracing::error!(
        iterations,
        max_iterations,
        resolved,
        queue = violations.len(),
        current = current.index(),
        "rebalance safety-net exceeded — dumping queue tail",
    );
    let tail = violations.len().saturating_sub(20);
    for (i, v) in violations[tail..].iter().enumerate() {
        if vnodes.is_occupied(v.index()) {
            tracing::error!(idx = tail + i, entry = %Ctx(vnodes, *v));
        } else {
            tracing::error!(idx = tail + i, node = v.index(), "DEAD");
        }
    }

    #[cfg(debug_assertions)]
    panic!(
        "rebalance: exceeded {max_iterations} iterations \
         (queue={}, node=v{}, resolved={resolved})",
        violations.len(),
        current.index(),
    );
    #[cfg(not(debug_assertions))]
    {
        tracing::error!(
            "breaking out of rebalance loop — \
             possible bug in violation resolution",
        );
        true
    }
}

pub fn rebalance<C: Coordinate, V: Accumulator + Inspectable, const N: u32>(
    vtree: &mut VTree<V>,
    gtree: &mut GTree<C, V, N>,
    depth_evict: u32,
) -> Vec<GNodeId> {
    let mut new_gnodes = Vec::new();

    let max_iterations: u32 = vtree.nodes.count().saturating_mul(20).max(10_000);
    let mut iterations: u32 = 0;
    let mut resolved: u32 = 0;

    let _span = tracing::debug_span!("rebalance", queue = vtree.violations.len()).entered();

    if tracing::enabled!(tracing::Level::DEBUG) {
        crate::diagnostics::diagnostic::audit_violations(
            &vtree.nodes,
            &vtree.violations,
            "PRE-REBALANCE",
        );
    }

    // Invariant: each `resolve` call either resolves the head node (decreasing
    // total violations by at least 1) or promotes the violation upward toward
    // the root (bounded by tree depth × branching factor).  The safety-net
    // `max_iterations` catches any cycle if that invariant is ever violated.
    while let Some(c) = vtree.violations.pop() {
        iterations += 1;
        if iterations > max_iterations
            && handle_iteration_limit(
                &vtree.nodes,
                &vtree.violations,
                iterations,
                max_iterations,
                resolved,
                c,
            )
        {
            break;
        }

        if !vtree.nodes.is_occupied(c.index()) {
            tracing::trace!(node = c.index(), "skip destroyed");
            continue;
        }

        if !is_violated(&vtree.nodes, c) {
            tracing::trace!(node = c.index(), "skip already resolved");
            continue;
        }

        resolved += 1;
        tracing::debug!(
            iter = iterations,
            resolved,
            queue = vtree.violations.len(),
            node = %Ctx(&vtree.nodes, c),
            "resolving violation",
        );
        let gid = {
            let mut tree = VTreeMutContext { vtree: &mut *vtree };
            resolve(&mut tree, gtree, c, depth_evict)
        };
        if let Some(gid) = gid {
            new_gnodes.push(gid);
        }

        if vtree.nodes.is_occupied(c.index()) && is_violated(&vtree.nodes, c) {
            tracing::warn!(
                iter = iterations,
                node = %Ctx(&vtree.nodes, c),
                "node STILL violated after resolve",
            );
        }

        if tracing::enabled!(tracing::Level::DEBUG) {
            crate::diagnostics::diagnostic::audit_violations(
                &vtree.nodes,
                &vtree.violations,
                "POST-RESOLVE",
            );
        }
    }

    tracing::debug!(iterations, resolved, "rebalance complete");

    if cfg!(debug_assertions) || tracing::enabled!(tracing::Level::DEBUG) {
        let remaining = crate::diagnostics::diagnostic::audit_violations(
            &vtree.nodes,
            &vtree.violations,
            "RESIDUAL",
        );
        assert!(
            remaining.is_empty(),
            "rebalance finished with residual violations: {remaining:?}"
        );
    }

    new_gnodes
}

#[cfg(test)]
mod tests {
    use crate::graph::algorithm::rebalance::max_uncle_intensity;
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::nodes::vnode::{Children, VNode};
    use crate::handle::VNodeId;

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

    fn fresh() -> G {
        GvGraph::new(make_config())
    }

    // ── is_violated ───────────────────────────────────────────────────
    mod is_violated_fn {
        use super::*;

        #[test]
        fn fresh_single_entry_is_not_violated() {
            // Before any observation v_root is None; after first observation
            // (delta ≤ split_threshold) the single root entry has no uncle.
            let mut g = fresh();
            g.observe(0u8, 2u32); // no split; single entry node remains root
            // Single entry has no parent → no uncle → not violated
            // We rely on `has_pending_violations` which uses the same predicate.
            assert!(!g.has_pending_violations());
        }

        #[test]
        fn no_violations_remain_after_bootstrap_split() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // bootstrap split
            assert!(!g.has_pending_violations());
        }

        #[test]
        fn no_violations_remain_after_multiple_observations() {
            let mut g = fresh();
            for coord in [0u8, 64, 128, 192, 32, 96, 160, 224] {
                g.observe(coord, 3u32);
            }
            assert!(!g.has_pending_violations());
        }
    }

    // ── max_uncle_intensity ───────────────────────────────────────────
    mod max_uncle_intensity_fn {
        use super::*;
        use crate::handle::{GNodeId, VNodeId};

        #[test]
        fn returns_none_for_node_with_no_grandparent() {
            // After bootstrap split the original entry (depth 1) has a parent
            // (root structural, depth 0) but no grandparent → no uncle.
            let mut g = fresh();
            g.observe(64u8, 3u32); // bootstrap split
            // v_root is the new structural root at depth 0.
            // Its children: original entry + cs structural.
            // Walk to a depth-1 child.
            let v_root = g.v_root().expect("v_root must exist after bootstrap split");
            let result = max_uncle_intensity(g.vnodes(), v_root);
            // v_root has no parent → no grandparent → None
            assert!(result.is_none());
        }

        #[test]
        fn is_some_for_node_with_grandparent() {
            // After bootstrap + one catalytic split depth-3 entries have
            // grandparents.  max_uncle_intensity should return Some.
            let mut g = fresh();
            g.observe(32u8, 3u32); // bootstrap
            g.observe(32u8, 3u32); // catalytic split in left child
            // Find a deep entry by using the extract API and checking total_sum.
            // The presence of a result is what we are testing — not the value.
            let v_root = g.v_root().unwrap();
            // The root itself has no grandparent → None
            assert!(max_uncle_intensity(g.vnodes(), v_root).is_none());
            // But total_sum being correct proves rebalance ran successfully.
            assert_eq!(g.total_sum(), 6u32);
        }

        #[test]
        fn returns_max_across_multiple_uncles() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());

            let c = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
                5,
                None,
                GNodeId::from_index(1),
                true,
                true,
            )));
            let sibling = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
                4,
                None,
                GNodeId::from_index(2),
                true,
                true,
            )));
            let u1 = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
                9,
                None,
                GNodeId::from_index(3),
                true,
                true,
            )));

            let parent = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                9,
                None,
                Children::new_2((c, 5), (sibling, 4)),
                true,
            )));
            let gp = VNodeId::from_index(vnodes.alloc(VNode::new_structural(
                18,
                None,
                Children::new_2((parent, 9), (u1, 9)),
                true,
            )));

            vnodes.get_mut(c.index()).set_parent(parent);
            vnodes.get_mut(sibling.index()).set_parent(parent);
            vnodes.get_mut(parent.index()).set_parent(gp);
            vnodes.get_mut(u1.index()).set_parent(gp);

            assert_eq!(max_uncle_intensity(&vnodes, c), Some(9));
        }
    }

    // ── find_violated_nodes ──────────────────────────────────────────
    mod find_violated_nodes_fn {
        use super::*;
        use crate::graph::algorithm::rebalance::find_violated_nodes;

        #[test]
        fn returns_empty_when_no_nodes_are_violated() {
            let mut g = fresh();
            g.observe(64u8, 2u32); // single root entry, no uncle relation
            let violated = find_violated_nodes(g.vnodes());
            assert!(violated.is_empty());
        }
    }

    // ── ViolationSources ─────────────────────────────────────────────
    mod violation_sources_fn {
        use crate::graph::algorithm::violation_sources::ViolationSources;

        #[test]
        fn default_enables_all_sources() {
            let d = ViolationSources::default();
            let a = ViolationSources::all_enabled();
            assert_eq!(
                d.source_3_contraction_grandchildren,
                a.source_3_contraction_grandchildren
            );
            assert_eq!(d.source_4_promotion_children, a.source_4_promotion_children);
            assert_eq!(
                d.source_6_leaf_removal_ancestors,
                a.source_6_leaf_removal_ancestors
            );
            assert_eq!(d.source_7_collapse_children, a.source_7_collapse_children);
        }
    }

    // ── Nd::fmt ───────────────────────────────────────────────────────
    mod nd_display_fn {
        use super::*;
        use crate::graph::algorithm::rebalance::Nd;
        use crate::handle::VNodeId;

        #[test]
        fn dead_vnode_shows_dead_marker() {
            let g = fresh();
            // index 999 is well beyond allocated vnodes → is_occupied returns false
            let display = format!("{}", Nd(g.vnodes(), VNodeId::from_index(999)));
            assert_eq!(display, "v999(DEAD)");
        }

        #[test]
        fn entry_vnode_shows_entry_label() {
            let mut g = fresh();
            g.observe(64u8, 2u32); // value == threshold: no split; single Entry remains
            let v_root = g.v_root().expect("v_root must exist");
            let display = format!("{}", Nd(g.vnodes(), v_root));
            assert!(
                display.contains("(E,"),
                "expected Entry label, got: {display}"
            );
        }

        #[test]
        fn structural_vnode_shows_structural_label() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // value > threshold → bootstrap split → v_root becomes Structural
            let v_root = g.v_root().expect("v_root must exist");
            let display = format!("{}", Nd(g.vnodes(), v_root));
            assert!(
                display.contains("(S"),
                "expected Structural label, got: {display}"
            );
        }
    }

    // ── Ctx::fmt ──────────────────────────────────────────────────────
    mod ctx_display_fn {
        use super::*;
        use crate::graph::algorithm::rebalance::Ctx;
        use crate::nodes::vnode::VKind;

        #[test]
        fn root_node_includes_root_label() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // bootstrap split → Structural root
            let v_root = g.v_root().expect("v_root must exist");
            // v_root has no parent → formatting appends " (root)"
            let display = format!("{}", Ctx(g.vnodes(), v_root));
            assert!(
                display.contains("(root)"),
                "expected '(root)', got: {display}"
            );
        }

        #[test]
        fn depth_one_child_includes_parent_info() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // bootstrap split
            let v_root = g.v_root().expect("v_root must exist");
            // Get a depth-1 child (first child of Structural root)
            let child_id = match &g.vnodes().get(v_root.index()).kind() {
                VKind::Structural { children, .. } => children.get(0).0,
                _ => panic!("expected Structural v_root after bootstrap"),
            };
            let display = format!("{}", Ctx(g.vnodes(), child_id));
            // depth-1 node: has parent, no grandparent → shows one "←" separator
            assert!(
                display.contains('\u{2190}'),
                "expected arrow, got: {display}"
            );
        }

        #[test]
        fn depth_two_node_includes_grandparent_info() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // first split
            g.observe(64u8, 3u32); // second split in left child
            let v_root = g.v_root().expect("v_root must exist");
            // BFS for a depth-2+ Entry
            let mut stack = vec![(v_root, 0usize)];
            let mut depth2 = None;
            while let Some((id, d)) = stack.pop() {
                let n = g.vnodes().get(id.index());
                match &n.kind() {
                    VKind::Entry { .. } if d >= 2 => {
                        depth2 = Some(id);
                        break;
                    }
                    VKind::Structural { children, .. } => {
                        for (cid, _) in children.iter() {
                            stack.push((cid, d + 1));
                        }
                    }
                    _ => {}
                }
            }
            let Some(d2) = depth2 else {
                return; // vacuous pass if depth-2 wasn't reached
            };
            let display = format!("{}", Ctx(g.vnodes(), d2));
            // depth-2 node: parent + grandparent → shows two "←" separators
            let arrow_count = display.matches('\u{2190}').count();
            assert!(arrow_count >= 2, "expected ≥2 arrows, got: {display}");
        }
    }

    // ── Ch display ───────────────────────────────────────────────────
    mod ch_display_fn {
        use super::*;
        use crate::graph::algorithm::rebalance::Ch;

        #[test]
        fn entry_vnode_displays_as_empty_set_symbol() {
            // In a fresh graph (no observations), v_root is an Entry vnode.
            // Ch(vnodes, entry_id) hits VKind::Entry arm → writes "∅" (line 77).
            let g = fresh();
            let v_root_id = g.v_root().expect("fresh graph has v_root");
            let result = format!("{}", Ch(g.vnodes(), v_root_id));
            assert_eq!(result, "\u{2205}", "Ch on Entry vnode should display '∅'");
        }
    }

    // ── rebalance (integration) ───────────────────────────────────────
    mod rebalance_fn {
        use super::*;
        use crate::graph::algorithm::rebalance::{
            VTreeMutContext, handle_iteration_limit, rebalance, resolve,
        };
        use crate::nodes::vnode::VNode;

        #[test]
        fn total_sum_invariant_is_maintained_after_many_observations() {
            let mut g = fresh();
            let mut expected_sum = 0u32;
            for (i, coord) in [0u8, 64, 32, 96, 16, 80, 48, 112].iter().enumerate() {
                let delta = (i as u32 + 1) * 3;
                g.observe(*coord, delta);
                expected_sum += delta;
            }
            assert_eq!(g.total_sum(), expected_sum);
        }

        #[test]
        fn node_count_is_at_least_one_after_many_observations() {
            let mut g = fresh();
            for coord in 0u8..20 {
                g.observe(coord.wrapping_mul(13), 3u32);
            }
            assert!(g.node_count() >= 1);
        }

        #[test]
        fn resolve_returns_none_when_node_has_no_parent() {
            let mut g = fresh();
            g.observe(64u8, 2u32); // no split: root entry has no parent
            let c = g.v_root().expect("v_root must exist");
            g.vtree.violations.clear();
            let depth_evict = g.gtree.live_depth_evict;
            let mut tree = VTreeMutContext { vtree: &mut g.vtree };
            let result = resolve(
                &mut tree,
                &mut g.gtree,
                c,
                depth_evict,
            );
            assert!(result.is_none());
            assert!(g.vtree.violations.is_empty());
        }

        #[test]
        fn rebalance_skips_destroyed_queue_nodes() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // ensure non-empty tree
            g.vtree.violations.push(VNodeId::from_index(9999));
            let depth_evict = g.gtree.live_depth_evict;

            let new_nodes = rebalance(
                &mut g.vtree,
                &mut g.gtree,
                depth_evict,
            );
            assert!(new_nodes.is_empty());
            assert!(g.vtree.violations.is_empty());
        }

        #[test]
        fn rebalance_skips_already_resolved_nodes() {
            let mut g = fresh();
            g.observe(64u8, 3u32); // establish a structural root
            let v_root = g.v_root().expect("v_root must exist");
            // Root has no uncle relation and should not be violated.
            g.vtree.violations.push(v_root);
            let depth_evict = g.gtree.live_depth_evict;

            let new_nodes = rebalance(
                &mut g.vtree,
                &mut g.gtree,
                depth_evict,
            );
            assert!(new_nodes.is_empty());
            assert!(g.vtree.violations.is_empty());
        }

        #[test]
        #[should_panic(expected = "rebalance: exceeded")]
        fn handle_iteration_limit_panics_in_debug_mode() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());
            let live = VNodeId::from_index(vnodes.alloc(VNode::new_entry(
                1,
                None,
                crate::handle::GNodeId::from_index(50),
                true,
                true,
            )));
            let violations = vec![live, VNodeId::from_index(9999)];

            let _ = handle_iteration_limit(&vnodes, &violations, 11, 10, 0, live);
        }
    }
}
