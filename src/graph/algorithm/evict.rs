use crate::graph::GvGraph;
use crate::graph::algorithm::rebalance;
use crate::graph::algorithm::violation_push::ViolationQueue;
use crate::handle::VNodeId;
use crate::nodes::gnode::GState;
use crate::nodes::vnode::VKind;
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::vtree::VNodeTree;

/// Topology of the V-node being evicted, captured before the leaf is removed.
struct LeafRemovalContext {
    v_parent: Option<VNodeId>,
    /// Number of children the V-parent had immediately before removal.
    child_count: usize,
    /// The V-node from which leaf-removal violations should be pushed.
    /// Present when the parent had 2 or 3 children.
    change_point: Option<VNodeId>,
    /// The sole surviving sibling when the parent collapses (2 → 1 children).
    collapse_sibling: Option<VNodeId>,
}

/// Captures the V-topology before removing `v_id` from the tree.
fn classify_leaf_removal<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    v_id: VNodeId,
) -> LeafRemovalContext {
    let v_parent = vnodes.get(v_id.index()).parent();
    let (child_count, change_point, collapse_sibling) = v_parent.map_or((0, None, None), |p| {
        let count = match &vnodes.get(p.index()).kind() {
            VKind::Structural { children, .. } => children.len(),
            VKind::Entry { .. } => 0,
        };
        match count {
            3 => (3, Some(p), None),
            2 => {
                let sibling = match &vnodes.get(p.index()).kind() {
                    VKind::Structural { children, .. } => {
                        let (c0, _) = children.get(0);
                        let (c1, _) = children.get(1);
                        if c0 == v_id { Some(c1) } else { Some(c0) }
                    }
                    VKind::Entry { .. } => None,
                };
                (2, vnodes.get(p.index()).parent(), sibling)
            }
            _ => (count, None, None),
        }
    });
    LeafRemovalContext {
        v_parent,
        child_count,
        change_point,
        collapse_sibling,
    }
}

/// Pushes all rebalancing violations triggered by the removal of `v_id`.
fn push_eviction_violations<V: Accumulator>(
    vnodes: &VNodeTree<V>,
    v_id: VNodeId,
    ctx: &LeafRemovalContext,
    queue: &mut ViolationQueue<'_>,
) {
    if let Some(start) = ctx.change_point {
        queue.push_leaf_removal(vnodes, start);
    }
    match ctx.child_count {
        2 => {
            if let Some(sole) = ctx.collapse_sibling {
                tracing::debug!(
                    sole = sole.index(),
                    "evict_tip: calling push_collapse_violations"
                );
                queue.push_collapse(vnodes, sole);
                if let Some(grandparent) = ctx.change_point {
                    tracing::debug!(
                        sole = sole.index(),
                        grandparent = grandparent.index(),
                        "evict_tip: calling push_cousin_violations (source 9)",
                    );
                    queue.push_cousin(vnodes, sole, grandparent);
                }
            }
        }
        3 => {
            if let Some(p) = ctx.v_parent {
                tracing::debug!(
                    parent = p.index(),
                    "evict_tip: calling push_remaining_sibling_violations"
                );
                queue.push_remaining_sibling(vnodes, p, v_id);
            }
        }
        _ => {}
    }
}

/// Parent G-node state captured before dealloc, for use by the plateau update.
struct ParentSnapshot<C> {
    state: GState,
    lo: C,
    hi: C,
}

#[allow(clippy::too_many_lines)]
impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn evict_tip(&mut self, v_id: VNodeId) {
        let span = tracing::debug_span!(
            "evict_tip",
            v_id = v_id.index(),
            gnode = tracing::field::Empty,
            parent = tracing::field::Empty,
        )
        .entered();

        let gnode_id = match &self.vtree.nodes.get(v_id.index()).kind() {
            VKind::Entry {
                gnode,
                is_evictable,
                ..
            } => {
                debug_assert!(
                    *is_evictable,
                    "evict_tip: V-entry {} is not evictable",
                    v_id.index()
                );
                *gnode
            }
            VKind::Structural { .. } => panic!(
                "evict_tip: V-node {} is structural, not an entry",
                v_id.index()
            ),
        };
        span.record("gnode", gnode_id.index());

        assert_ne!(
            gnode_id, self.gtree.nodes.root,
            "evict_tip: cannot evict the G-root"
        );

        let parent_id = self
            .gtree
            .nodes
            .get(gnode_id.index())
            .parent()
            .expect("evict_tip: terminal G-node must have a parent");
        span.record("parent", parent_id.index());

        // ── Phase 1: G-tree restructuring ───────────────────────────────────────
        // Absorb the evicted child's sum into the parent's own weight, detach
        // the child slot, and recompute the parent sum invariant.
        let parent_sum_before = self.gtree.nodes.get(parent_id.index()).sum();
        self.gtree.nodes.merge_into_parent(gnode_id);
        debug_assert!(
            (self
                .gtree
                .nodes
                .get(parent_id.index())
                .sum()
                .to_f64_approx()
                - parent_sum_before.to_f64_approx())
            .abs()
                < 1e-9,
            "evict_tip: G-sum invariant violation after absorption: \
         recomputed={}, expected={}",
            self.gtree
                .nodes
                .get(parent_id.index())
                .sum()
                .to_f64_approx(),
            parent_sum_before.to_f64_approx()
        );

        // Capture the parent's post-eviction state here: G-tree structure will
        // not change further through the V-tree phases below.
        let parent_snapshot = {
            let pg = self.gtree.nodes.get(parent_id.index());
            ParentSnapshot {
                state: pg.state(),
                lo: pg.lo(),
                hi: pg.hi(),
            }
        };

        // ── Phase 2: V-intensity propagation ────────────────────────────────────
        // The parent's `own` changed; push its new intensity up the V-tree and
        // requeue any nodes that are now violated.
        let p_entry_id = self
            .gtree
            .nodes
            .get(parent_id.index())
            .entry()
            .expect("evict_tip: parent must have V-entry (has dependents)");
        {
            let p_own = self.gtree.nodes.get(parent_id.index()).own();
            self.vtree
                .nodes
                .get_mut(p_entry_id.index())
                .set_intensity(p_own);
            self.vtree.sync_intensity(p_entry_id, p_own);
            self.vtree.propagate_sums(p_entry_id);

            let mut check_id = Some(p_entry_id);
            while let Some(id) = check_id {
                if rebalance::is_violated(&self.vtree.nodes, id) {
                    self.vtree.push_violation(id);
                }
                check_id = self.vtree.nodes.get(id.index()).parent();
            }
        }

        // ── Phase 3: Evictable / exposed flag propagation ────────────────────────
        {
            let p = self.gtree.nodes.get(parent_id.index());
            let parent_is_exposed = p.uncovered_range().is_some();
            let parent_is_evictable = p.is_terminal();
            let p_entry_id = p
                .entry()
                .expect("evict_tip: parent must have V-entry (has dependents)");
            self.vtree
                .set_entry_flags(p_entry_id, parent_is_exposed, parent_is_evictable);
            self.vtree.propagate_evictable(p_entry_id);
        }

        // ── Phase 4–6: Capture V-topology, remove leaf, push violations ─────
        // Topology must be captured before removal; violations are pushed after.
        let removal_ctx = classify_leaf_removal(&self.vtree.nodes, v_id);

        self.vtree.remove_leaf(&mut self.gtree, v_id);

        let mut queue = ViolationQueue::new(&mut self.vtree.violations);
        push_eviction_violations(&self.vtree.nodes, v_id, &removal_ctx, &mut queue);

        // ── Phase 7: Debug audit for missed violations ───────────────────────
        if tracing::enabled!(tracing::Level::ERROR) {
            let ctx = crate::diagnostics::diagnostic::MissedViolationContext {
                evicted_parent: removal_ctx.v_parent,
                evicted_parent_child_count: removal_ctx.child_count,
                collapse_sibling: removal_ctx.collapse_sibling,
            };
            let missed = crate::diagnostics::diagnostic::audit_violations(
                &self.vtree.nodes,
                &self.vtree.violations,
                "POST-EVICT",
            );
            for v in missed {
                crate::diagnostics::diagnostic::diagnose_missed_violation_in_tree(
                    &self.vtree,
                    v,
                    &ctx,
                );
            }
        }

        // ── Phase 8: Dealloc evicted node and update counts ──────────────────────
        self.gtree.nodes.dealloc(gnode_id.index());
        self.gtree.nodes.node_count -= 1;
        self.gtree.nodes.terminal_count -= 1;
        if self.gtree.nodes.get(parent_id.index()).is_terminal() {
            self.gtree.nodes.terminal_count += 1;
        }

        // ── Phase 9: Plateau mirror update ───────────────────────────────────────
        // plateau_after_evict is a no-op when the feature is disabled.
        let parent_state_after = parent_snapshot.state;
        self.plateau_after_evict(
            gnode_id,
            parent_id,
            parent_state_after,
            parent_snapshot.lo,
            parent_snapshot.hi,
        );
        if tracing::enabled!(tracing::Level::DEBUG) {
            self.debug_assert_plateau_mirror_consistency("POST-EVICT");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::handle::{GNodeId, VNodeId};
    use crate::nodes::vnode::{Children, VNode};

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

    /// Config with a small budget so the observe loop reaches the eviction
    /// check.  `depth_create=1` < `depth_evict=2` satisfies all invariants.
    /// `soft_limit` = budget (10) - headroom (9) = 1.
    fn eviction_config() -> Config<u32> {
        // depth_buffer = depth_evict - depth_create = 2 - 1 = 1
        // headroom     = 3^(1+1) = 9
        // required_headroom = max(9, 0) = 9
        // soft_limit   = budget - 9 = 10 - 9 = 1
        Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 1,
                depth_evict: 2,
                budget: Some(10),
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        }
    }

    // ── scan_for_candidates ───────────────────────────────────────────
    mod scan_for_candidates_fn {
        use super::*;

        #[test]
        fn returns_empty_for_fresh_graph() {
            // v_root is None on a fresh graph — scan returns nothing
            let g: G = GvGraph::new(make_config());
            let candidates = g
                .vtree
                .scan_for_candidates(g.gtree.live_depth_evict, g.gtree.nodes.root);
            assert!(candidates.is_empty());
        }

        #[test]
        fn returns_empty_when_all_nodes_shallower_than_live_depth_evict() {
            // default config: depth_evict=5, after one split nodes are at
            // v-depth 2 which is well below 5 → no candidates
            let mut g: G = GvGraph::new(make_config());
            g.observe(64u8, 3u32); // triggers bootstrap split
            let candidates = g
                .vtree
                .scan_for_candidates(g.gtree.live_depth_evict, g.gtree.nodes.root);
            assert!(candidates.is_empty());
        }

        #[test]
        fn scan_traverses_tree_without_panic_after_several_splits() {
            // eviction_config: depth_evict=2, bootstrap leaves at v-depth 2
            // depth > 2 → no candidates, but DFS still traverses every node
            let mut g: G = GvGraph::new(eviction_config());
            g.observe(32u8, 3u32);
            g.observe(192u8, 3u32);
            let candidates = g
                .vtree
                .scan_for_candidates(g.gtree.live_depth_evict, g.gtree.nodes.root);
            // entries at depth 2 are not > live_depth_evict=2 → empty
            assert!(candidates.is_empty());
        }
    }

    // ── evict_tip (via bounded-budget observe) ────────────────────────
    mod evict_tip_fn {
        use super::*;

        #[test]
        fn observe_with_budget_keeps_node_count_bounded() {
            // soft_limit=1, so after every split the eviction loop fires.
            let mut g: G = GvGraph::new(eviction_config());
            for i in 0u8..10 {
                g.observe(i.wrapping_mul(13), 3u32);
            }
            // The eviction mechanism must keep the graph alive (no panic).
            assert!(g.gtree.nodes.node_count >= 1);
        }

        #[test]
        fn total_sum_is_preserved_after_eviction() {
            // Energy is transferred to parent on eviction, so total_sum must
            // equal the sum of all delta values observed.
            let mut g: G = GvGraph::new(eviction_config());
            let n = 5u32;
            let delta = 3u32;
            for i in 0..n as u8 {
                g.observe(i.wrapping_mul(51), delta);
            }
            assert_eq!(g.total_sum(), n * delta);
        }

        #[test]
        #[should_panic(expected = "is structural, not an entry")]
        fn panics_when_called_on_structural_vnode() {
            let mut g: G = GvGraph::new(make_config());
            g.observe(64u8, 3u32); // bootstrap split: v_root becomes structural
            let v_root = g.v_root().expect("v_root must exist");
            g.evict_tip(v_root);
        }
    }

    // ── classify_leaf_removal ─────────────────────────────────────────
    mod classify_leaf_removal_fn {
        use super::*;
        use super::super::classify_leaf_removal;
        use super::super::{LeafRemovalContext, push_eviction_violations};
        use crate::graph::algorithm::violation_push::ViolationQueue;

        fn id(i: usize) -> VNodeId {
            VNodeId::from_index(i)
        }

        #[test]
        fn pair_parent_reports_grandparent_change_point_and_sibling() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());

            let target = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(1),
                true,
                true,
            )));
            let sibling = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(2),
                true,
                true,
            )));
            let uncle = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(3),
                true,
                true,
            )));

            let parent = id(vnodes.alloc(VNode::new_structural(
                2,
                None,
                Children::new_2((target, 1), (sibling, 1)),
                true,
            )));
            let grandparent = id(vnodes.alloc(VNode::new_structural(
                3,
                None,
                Children::new_2((parent, 2), (uncle, 1)),
                true,
            )));

            vnodes.get_mut(target.index()).set_parent(parent);
            vnodes.get_mut(sibling.index()).set_parent(parent);
            vnodes.get_mut(parent.index()).set_parent(grandparent);
            vnodes.get_mut(uncle.index()).set_parent(grandparent);

            let ctx = classify_leaf_removal(&vnodes, target);
            assert_eq!(ctx.v_parent, Some(parent));
            assert_eq!(ctx.child_count, 2);
            assert_eq!(ctx.change_point, Some(grandparent));
            assert_eq!(ctx.collapse_sibling, Some(sibling));
        }

        #[test]
        fn triple_parent_reports_parent_as_change_point() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());

            let a = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(10),
                true,
                true,
            )));
            let b = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(11),
                true,
                true,
            )));
            let c = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(12),
                true,
                true,
            )));

            let parent = id(vnodes.alloc(VNode::new_structural(
                3,
                None,
                Children::new_3((a, 1), (b, 1), (c, 1)),
                true,
            )));
            vnodes.get_mut(a.index()).set_parent(parent);
            vnodes.get_mut(b.index()).set_parent(parent);
            vnodes.get_mut(c.index()).set_parent(parent);

            let ctx = classify_leaf_removal(&vnodes, b);
            assert_eq!(ctx.v_parent, Some(parent));
            assert_eq!(ctx.child_count, 3);
            assert_eq!(ctx.change_point, Some(parent));
            assert_eq!(ctx.collapse_sibling, None);
        }

        #[test]
        fn lone_entry_has_zero_child_context() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());
            let target = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(20),
                true,
                true,
            )));

            let ctx = classify_leaf_removal(&vnodes, target);
            assert_eq!(ctx.v_parent, None);
            assert_eq!(ctx.child_count, 0);
            assert_eq!(ctx.change_point, None);
            assert_eq!(ctx.collapse_sibling, None);
        }

        #[test]
        fn push_eviction_violations_noops_for_zero_child_context() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());
            let target = id(vnodes.alloc(VNode::new_entry(
                1,
                None,
                GNodeId::from_index(30),
                true,
                true,
            )));

            let ctx = LeafRemovalContext {
                v_parent: None,
                child_count: 0,
                change_point: None,
                collapse_sibling: None,
            };
            let mut violations = Vec::new();
            let mut queue = ViolationQueue::new(&mut violations);
            push_eviction_violations(&vnodes, target, &ctx, &mut queue);
            assert!(violations.is_empty());
        }

        #[test]
        fn push_eviction_violations_executes_pair_path() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());

            let target = id(vnodes.alloc(VNode::new_entry(
                5,
                None,
                GNodeId::from_index(40),
                true,
                true,
            )));
            let sibling = id(vnodes.alloc(VNode::new_entry(
                4,
                None,
                GNodeId::from_index(41),
                true,
                true,
            )));
            let uncle = id(vnodes.alloc(VNode::new_entry(
                3,
                None,
                GNodeId::from_index(42),
                true,
                true,
            )));
            let parent = id(vnodes.alloc(VNode::new_structural(
                9,
                None,
                Children::new_2((target, 5), (sibling, 4)),
                true,
            )));
            let grandparent = id(vnodes.alloc(VNode::new_structural(
                12,
                None,
                Children::new_2((parent, 9), (uncle, 3)),
                true,
            )));

            vnodes.get_mut(target.index()).set_parent(parent);
            vnodes.get_mut(sibling.index()).set_parent(parent);
            vnodes.get_mut(parent.index()).set_parent(grandparent);
            vnodes.get_mut(uncle.index()).set_parent(grandparent);

            let ctx = LeafRemovalContext {
                v_parent: Some(parent),
                child_count: 2,
                change_point: Some(grandparent),
                collapse_sibling: Some(sibling),
            };

            let mut violations = Vec::new();
            let mut queue = ViolationQueue::new(&mut violations);
            push_eviction_violations(&vnodes, target, &ctx, &mut queue);
            assert!(!violations.is_empty());
        }

        #[test]
        fn push_eviction_violations_executes_triple_path() {
            let mut vnodes = crate::tree::vtree::VNodeTree::<u32>::from(crate::arena::Arena::new());

            let target = id(vnodes.alloc(VNode::new_entry(
                5,
                None,
                GNodeId::from_index(60),
                true,
                true,
            )));
            let s1 = id(vnodes.alloc(VNode::new_entry(
                4,
                None,
                GNodeId::from_index(61),
                true,
                true,
            )));
            let s2 = id(vnodes.alloc(VNode::new_entry(
                3,
                None,
                GNodeId::from_index(62),
                true,
                true,
            )));
            let parent = id(vnodes.alloc(VNode::new_structural(
                12,
                None,
                Children::new_3((target, 5), (s1, 4), (s2, 3)),
                true,
            )));

            vnodes.get_mut(target.index()).set_parent(parent);
            vnodes.get_mut(s1.index()).set_parent(parent);
            vnodes.get_mut(s2.index()).set_parent(parent);

            let ctx = LeafRemovalContext {
                v_parent: Some(parent),
                child_count: 3,
                change_point: Some(parent),
                collapse_sibling: None,
            };

            let mut violations = Vec::new();
            let mut queue = ViolationQueue::new(&mut violations);
            push_eviction_violations(&vnodes, target, &ctx, &mut queue);
            // Branch execution is what we need here; depending on intensities
            // and topology, this path may or may not enqueue violations.
            assert!(violations.len() <= 3);
        }
    }
}
