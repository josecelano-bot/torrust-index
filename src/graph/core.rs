use crate::graph::algorithm::rebalance;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::{Children, VKind, VNode};
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::gtree::GTree;
use crate::tree::vtree::{VNodeTree, VTree};

/// The dual-tree core of a `GvGraph`.
///
/// Owns the G-tree (geometric partition skeleton) and the V-tree
/// (intensity-aggregation overlay).  Operations that span both trees belong
/// here; operations that also need `config` or the plateau `tracker` live on
/// `GvGraph` instead.
pub struct GvCore<C: Coordinate, V: Accumulator, const N: u32> {
    pub(crate) gtree: GTree<C, V, N>,
    pub(crate) vtree: VTree<V>,
}

impl<C: Coordinate, V: Accumulator, const N: u32> Clone for GvCore<C, V, N> {
    fn clone(&self) -> Self {
        Self {
            gtree: self.gtree.clone(),
            vtree: self.vtree.clone(),
        }
    }
}

impl<C: Coordinate, V: Accumulator, const N: u32> GvCore<C, V, N> {
    pub(crate) fn alloc_v_entry(&mut self, gnode: GNodeId) -> VNodeId {
        let e = VNode::new_entry(V::zero(), None, gnode, true, true);
        let e_id = VNodeId::from_index(self.vtree.nodes.alloc(e).0);
        self.gtree.nodes.assign_entry(gnode, e_id);
        e_id
    }

    /// Promotes entry `c` from a semi-internal G-node into the grandparent level of
    /// the V-tree, creating the missing G-child that `c` previously occupied.
    ///
    /// # TODO
    ///
    /// This function is a legacy adaptation path used during the transition away from
    /// the old borrow-split resolve model.  Once `resolve_path_b` is refactored to
    /// operate directly on `&mut GvCore`, this function should be inlined or removed.
    pub(crate) fn legacy_promote(&mut self, c: VNodeId) -> GNodeId {
        let p = self
            .vtree
            .nodes
            .get(c.index())
            .parent()
            .expect("legacy_promote: c must have a parent");
        let vg = self
            .vtree
            .nodes
            .get(p.index())
            .parent()
            .expect("legacy_promote: p must have a grandparent");
        let gnode_id = match &self.vtree.nodes.get(c.index()).kind() {
            VKind::Entry { gnode, .. } => *gnode,
            VKind::Structural { .. } => panic!("legacy_promote: c must be an entry"),
        };
        debug_assert!(
            self.gtree.nodes.get(gnode_id.index()).is_semi_internal(),
            "legacy_promote: backing G-node must be semi-internal"
        );

        let _span = tracing::debug_span!(
            "legacy_promote",
            c = %rebalance::Nd(&self.vtree.nodes, c),
            p = p.index(),
            vg = vg.index(),
            gnode = gnode_id.index(),
        )
        .entered();

        // Create the missing G-child and its paired V-entry under p
        let (new_child_id, ne_id) = self.create_gchild_and_ventry(gnode_id, p);

        // Swap ne into p in place of c, preserving c's intensity for the lift
        let c_int = self.vtree.nodes.get(c.index()).intensity();
        self.vtree.replace_structural_child(p, c, ne_id, V::zero());

        // Lift c up to vg as a third sibling
        self.lift_c_to_grandparent(c, c_int, p, vg);

        // Clear c's flags and propagate evictability upward
        self.demote_and_propagate(c, p, vg);

        tracing::debug!(
            new_gnode = new_child_id.index(),
            new_ventry = ne_id.index(),
            "legacy_promote complete: c lifted to grandparent, new child created",
        );

        new_child_id
    }

    fn create_gchild_and_ventry(
        &mut self,
        gnode_id: GNodeId,
        parent_v: VNodeId,
    ) -> (GNodeId, VNodeId) {
        let new_child_id = self.gtree.nodes.allocate_missing_child(gnode_id);
        let ne = VNode::new_entry(V::zero(), Some(parent_v), new_child_id, true, true);
        let ne_id = VNodeId::from_index(self.vtree.nodes.alloc(ne).0);
        self.gtree.nodes.assign_entry(new_child_id, ne_id);
        (new_child_id, ne_id)
    }

    fn lift_c_to_grandparent(&mut self, c: VNodeId, c_int: V, p: VNodeId, vg: VNodeId) {
        let (u_id, u_int) = self.vtree.nodes.sibling_of(vg, p);
        // c is being demoted — it is never evictable at this point
        let p_evictable = self.vtree.nodes.node_has_evictable(p);
        let u_evictable = self.vtree.nodes.node_has_evictable(u_id);
        let p_int = self.vtree.nodes.get(p.index()).intensity();
        let g_node = self.vtree.nodes.get_mut(vg.index());
        if let VKind::Structural {
            children,
            has_evictable,
        } = g_node.kind_mut()
        {
            *children = Children::new_3((c, c_int), (p, p_int), (u_id, u_int));
            *has_evictable = p_evictable || u_evictable;
        }
        self.vtree.nodes.get_mut(c.index()).set_parent(vg);
    }

    fn demote_and_propagate(&mut self, c: VNodeId, p: VNodeId, vg: VNodeId) {
        self.vtree.set_entry_flags(c, false, false);
        self.vtree.recompute_and_sync(p);
        self.vtree.propagate_evictable(p);
        self.vtree.propagate_evictable(vg);
    }
}

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvCore<C, V, N> {
    pub(crate) fn rebalance(&mut self) -> Vec<GNodeId> {
        use crate::diagnostics::diagnostic::audit_violations;

        let depth_evict = self.gtree.live_depth_evict;
        let mut new_gnodes = Vec::new();

        let max_iterations: u32 = self.vtree.nodes.count().saturating_mul(20).max(10_000);
        let mut iterations: u32 = 0;
        let mut resolved: u32 = 0;

        let _span =
            tracing::debug_span!("rebalance", queue = self.vtree.violations.len()).entered();

        if tracing::enabled!(tracing::Level::DEBUG) {
            audit_violations(&self.vtree.nodes, &self.vtree.violations, "PRE-REBALANCE");
        }

        // Invariant: each `resolve` call either resolves the head node (decreasing
        // total violations by at least 1) or promotes the violation upward toward
        // the root (bounded by tree depth × branching factor).  The safety-net
        // `max_iterations` catches any cycle if that invariant is ever violated.
        while let Some(c) = self.vtree.violations.pop() {
            iterations += 1;
            if iterations > max_iterations {
                // In debug builds handle_iteration_limit diverges via panic!;
                // in release it logs and signals break.
                if handle_iteration_limit(
                    &self.vtree.nodes,
                    &self.vtree.violations,
                    iterations,
                    max_iterations,
                    resolved,
                    c,
                ) {
                    break;
                }
            }

            if !self.vtree.nodes.is_occupied(c.index()) {
                tracing::trace!(node = c.index(), "skip destroyed");
                continue;
            }

            if !rebalance::is_violated(&self.vtree.nodes, c) {
                tracing::trace!(node = c.index(), "skip already resolved");
                continue;
            }

            resolved += 1;
            tracing::debug!(
                iter = iterations,
                resolved,
                queue = self.vtree.violations.len(),
                node = %rebalance::Ctx(&self.vtree.nodes, c),
                "resolving violation",
            );
            let gid = rebalance::resolve(self, c, depth_evict);
            if let Some(gid) = gid {
                new_gnodes.push(gid);
            }

            if self.vtree.nodes.is_occupied(c.index())
                && rebalance::is_violated(&self.vtree.nodes, c)
            {
                tracing::warn!(
                    iter = iterations,
                    node = %rebalance::Ctx(&self.vtree.nodes, c),
                    "node STILL violated after resolve",
                );
            }

            if tracing::enabled!(tracing::Level::DEBUG) {
                audit_violations(&self.vtree.nodes, &self.vtree.violations, "POST-RESOLVE");
            }
        }

        tracing::debug!(iterations, resolved, "rebalance complete");

        if cfg!(debug_assertions) || tracing::enabled!(tracing::Level::DEBUG) {
            let remaining = audit_violations(&self.vtree.nodes, &self.vtree.violations, "RESIDUAL");
            assert!(
                remaining.is_empty(),
                "rebalance finished with residual violations: {remaining:?}"
            );
        }

        new_gnodes
    }
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
            tracing::error!(idx = tail + i, entry = %rebalance::Ctx(vnodes, *v));
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

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::handle::{GNodeId, VNodeId};
    use crate::nodes::vnode::{Children, VKind, VNode};

    type G = GvGraph<u8, u32, 8>;

    fn base_structural() -> StructuralConfig {
        StructuralConfig {
            depth_create: 3,
            depth_evict: 5,
            budget: None,
            alpha_relax: 0.5,
            bounded_eviction: false,
        }
    }

    fn make_config() -> Config<u32> {
        Config {
            split_threshold: 2,
            structural: base_structural(),
        }
    }

    fn fresh() -> G {
        GvGraph::new(make_config())
    }

    fn make_graph() -> GvGraph<u8, u64, 8> {
        GvGraph::new(Config {
            split_threshold: 2,
            structural: base_structural(),
        })
    }

    // ── legacy_promote ────────────────────────────────────────────────

    #[test]
    #[allow(clippy::too_many_lines)]
    fn legacy_promote_lifts_entry_and_creates_missing_gchild() {
        let mut graph = make_graph();

        let semi_gid = graph.core.gtree.nodes.root;
        let existing_child = graph.core.gtree.nodes.allocate_missing_child(semi_gid);
        assert!(
            graph
                .core
                .gtree
                .nodes
                .get(semi_gid.index())
                .is_semi_internal()
        );

        let c = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_entry(7, None, semi_gid, true, true))
                .0,
        );
        let s = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_entry(
                    5,
                    None,
                    GNodeId::from_index(existing_child.index()),
                    true,
                    true,
                ))
                .0,
        );
        let u = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_entry(
                    11,
                    None,
                    GNodeId::from_index(existing_child.index()),
                    true,
                    true,
                ))
                .0,
        );

        let p = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    12,
                    None,
                    Children::new_2((c, 7), (s, 5)),
                    true,
                ))
                .0,
        );
        graph.core.vtree.nodes.get_mut(c.index()).set_parent(p);
        graph.core.vtree.nodes.get_mut(s.index()).set_parent(p);

        let g = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    23,
                    None,
                    Children::new_2((p, 12), (u, 11)),
                    true,
                ))
                .0,
        );
        graph.core.vtree.nodes.get_mut(p.index()).set_parent(g);
        graph.core.vtree.nodes.get_mut(u.index()).set_parent(g);

        graph.core.gtree.nodes.assign_entry(semi_gid, c);

        let new_gid = graph.core.legacy_promote(c);
        let new_entry = graph
            .core
            .gtree
            .nodes
            .get(new_gid.index())
            .entry()
            .expect("new G-child must have V-entry");

        let semi = graph.core.gtree.nodes.get(semi_gid.index());
        assert!(semi.left().is_some());
        assert!(semi.right().is_some());

        match graph.core.vtree.nodes.get(p.index()).kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(children.get(0).0, new_entry);
                assert_eq!(children.get(1).0, s);
            }
            VKind::Entry { .. } => panic!("p should remain structural"),
        }

        match graph.core.vtree.nodes.get(g.index()).kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(children.len(), 3);
                assert_eq!(children.get(0).0, c);
                assert_eq!(children.get(1).0, p);
                assert_eq!(children.get(2).0, u);
            }
            VKind::Entry { .. } => panic!("g should remain structural"),
        }

        match graph.core.vtree.nodes.get(c.index()).kind() {
            VKind::Entry {
                is_exposed,
                is_evictable,
                ..
            } => {
                assert!(!is_exposed);
                assert!(!is_evictable);
            }
            VKind::Structural { .. } => panic!("c should remain entry"),
        }

        match graph.core.vtree.nodes.get(new_entry.index()).kind() {
            VKind::Entry {
                gnode,
                is_exposed,
                is_evictable,
            } => {
                assert_eq!(*gnode, new_gid);
                assert!(*is_exposed);
                assert!(*is_evictable);
            }
            VKind::Structural { .. } => panic!("new entry must be an entry"),
        }

        assert_eq!(graph.core.vtree.nodes.get(c.index()).parent(), Some(g));
        assert_eq!(
            graph.core.vtree.nodes.get(new_entry.index()).parent(),
            Some(p)
        );
        assert_eq!(
            graph.core.gtree.nodes.get(new_gid.index()).parent(),
            Some(semi_gid)
        );
    }

    #[test]
    #[should_panic(expected = "legacy_promote: c must be an entry")]
    fn legacy_promote_panics_for_structural_c() {
        let mut graph = make_graph();

        let semi_gid = graph.core.gtree.nodes.root;
        let _ = graph.core.gtree.nodes.allocate_missing_child(semi_gid);

        let c1 = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_entry(1, None, semi_gid, true, true))
                .0,
        );
        let c2 = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_entry(2, None, semi_gid, true, true))
                .0,
        );
        let c = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    3,
                    None,
                    Children::new_2((c1, 1), (c2, 2)),
                    true,
                ))
                .0,
        );
        graph.core.vtree.nodes.get_mut(c1.index()).set_parent(c);
        graph.core.vtree.nodes.get_mut(c2.index()).set_parent(c);

        let u = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_entry(9, None, semi_gid, true, true))
                .0,
        );
        let p = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    3,
                    None,
                    Children::new_2((c, 3), (u, 9)),
                    true,
                ))
                .0,
        );
        graph.core.vtree.nodes.get_mut(c.index()).set_parent(p);
        graph.core.vtree.nodes.get_mut(u.index()).set_parent(p);

        let g = VNodeId::from_index(
            graph
                .core
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    12,
                    None,
                    Children::new_2((p, 12), (u, 9)),
                    true,
                ))
                .0,
        );
        graph.core.vtree.nodes.get_mut(p.index()).set_parent(g);

        let _ = graph.core.legacy_promote(c);
    }

    // ── rebalance ─────────────────────────────────────────────────────

    mod rebalance_fn {
        // Black-box tests: exercise GvGraph's public API only.
        // These should survive any internal restructuring.
        mod public_api {
            use super::super::*;

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
            fn rebalance_skips_destroyed_queue_nodes() {
                let mut g = fresh();
                g.observe(64u8, 3u32); // ensure non-empty tree
                g.core.vtree.violations.push(VNodeId::from_index(9999));

                let new_nodes = g.core.rebalance();
                assert!(new_nodes.is_empty());
                assert!(g.core.vtree.violations.is_empty());
            }

            #[test]
            fn rebalance_skips_already_resolved_nodes() {
                let mut g = fresh();
                g.observe(64u8, 3u32); // establish a structural root
                let v_root = g.v_root().expect("v_root must exist");
                // Root has no uncle relation and should not be violated.
                g.core.vtree.violations.push(v_root);

                let new_nodes = g.core.rebalance();
                assert!(new_nodes.is_empty());
                assert!(g.core.vtree.violations.is_empty());
            }
        }

        // White-box tests: exercise internal helpers directly.
        // These are coupled to the current implementation; update them when refactoring internals.
        mod internals {
            use super::super::*;
            use crate::graph::algorithm::rebalance::resolve;
            use crate::nodes::vnode::VNode;

            #[test]
            fn resolve_returns_none_when_node_has_no_parent() {
                let mut g = fresh();
                g.observe(64u8, 2u32); // no split: root entry has no parent
                let c = g.v_root().expect("v_root must exist");
                g.core.vtree.violations.clear();
                let depth_evict = g.core.gtree.live_depth_evict;
                let result = resolve(&mut g.core, c, depth_evict);
                assert!(result.is_none());
                assert!(g.core.vtree.violations.is_empty());
            }

            #[test]
            #[should_panic(expected = "rebalance: exceeded")]
            fn handle_iteration_limit_panics_in_debug_mode() {
                // Note: the release-build path (returns true, signals break) is not covered by
                // automated tests — exercising it requires a cycle in violation resolution.
                let mut vnodes =
                    crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());
                let live = VNodeId::from_index(
                    vnodes
                        .alloc(VNode::new_entry(
                            1,
                            None,
                            crate::handle::GNodeId::from_index(50),
                            true,
                            true,
                        ))
                        .0,
                );
                let violations = vec![live, VNodeId::from_index(9999)];

                let _ = super::super::super::handle_iteration_limit(
                    &vnodes,
                    &violations,
                    11,
                    10,
                    0,
                    live,
                );
            }
        }
    }
}
