use crate::graph::GvGraph;
use crate::handle::GNodeId;
use crate::traits::{Accumulator, Coordinate, Inspectable, PlateauTracking};

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn handle_legacy_promotes(&mut self, new_gnodes: &[GNodeId]) {
        for &_new_gid in new_gnodes {
            self.core.gtree.nodes.node_count += 1;

            self.core.gtree.nodes.terminal_count += 1;
        }
        self.plateau_after_legacy_promotes_batched(new_gnodes);
    }

    pub(crate) fn adjust_depth_gates(&mut self) {
        let Some(budget) = self.config.structural.budget else {
            return;
        };
        let count = self.core.gtree.nodes.node_count as usize;

        let convergence_bound = 2 * (self.core.gtree.live_depth_create as usize).saturating_sub(1);
        let required_headroom = self.core.gtree.headroom.max(convergence_bound);
        let soft_limit = budget - required_headroom;
        assert!(
            soft_limit >= 1,
            "soft_limit must be >= 1 (budget={budget}, headroom={required_headroom}, \
             D_c={}, buffer={})",
            self.core.gtree.live_depth_create,
            self.core.gtree.depth_buffer
        );
        self.core.gtree.soft_limit = Some(soft_limit);

        if count > soft_limit {
            let floor = self.core.gtree.depth_buffer + 1;
            if self.core.gtree.live_depth_evict > floor {
                self.core.gtree.live_depth_evict -= 1;
                self.core.gtree.live_depth_create =
                    self.core.gtree.live_depth_evict - self.core.gtree.depth_buffer;
                tracing::debug!(
                    new_d_evict = self.core.gtree.live_depth_evict,
                    new_d_create = self.core.gtree.live_depth_create,
                    count,
                    soft_limit,
                    "depth gates tightened",
                );
            }
        } else {
            #[allow(clippy::cast_precision_loss)]
            let threshold = soft_limit as f64 * self.config.structural.alpha_relax;
            #[allow(clippy::cast_precision_loss)]
            let count_f = count as f64;
            if count_f < threshold {
                self.core.gtree.live_depth_evict += 1;
                self.core.gtree.live_depth_create =
                    self.core.gtree.live_depth_evict - self.core.gtree.depth_buffer;
                tracing::debug!(
                    new_d_evict = self.core.gtree.live_depth_evict,
                    new_d_create = self.core.gtree.live_depth_create,
                    count,
                    soft_limit,
                    "depth gates relaxed",
                );
            }
        }
    }

    pub fn check_evictions(&mut self) -> u32 {
        let _span = tracing::debug_span!(
            "check_evictions",
            d_evict = self.core.gtree.live_depth_evict
        )
        .entered();
        self.evict_candidates(None)
    }

    pub(crate) fn check_evictions_bounded(&mut self, stop_at: usize) -> u32 {
        self.evict_candidates(Some(stop_at))
    }

    /// Returns `true` when `v_id` is a live, evictable entry-node whose V-tree
    /// depth is above the current eviction gate.  Guards the inner loop in
    /// `evict_candidates` without nesting.
    fn is_eviction_candidate(&self, v_id: crate::handle::VNodeId) -> bool {
        use crate::tree::vtree::vnode::VKind;
        if !self.core.vtree.nodes.is_occupied(v_id.index()) {
            return false;
        }
        match &self.core.vtree.nodes.get(v_id.index()).kind() {
            VKind::Structural { .. } => false,
            VKind::Entry {
                gnode,
                is_evictable,
                ..
            } => {
                *is_evictable
                    && *gnode != self.core.gtree.nodes.root
                    && self.core.vtree.depth(v_id) > self.core.gtree.live_depth_evict
            }
        }
    }

    fn evict_candidates(&mut self, stop_at: Option<usize>) -> u32 {
        let candidates = self
            .core
            .vtree
            .scan_for_candidates(self.core.gtree.live_depth_evict, self.core.gtree.nodes.root);
        let _span =
            tracing::debug_span!("evict_batch", candidate_count = candidates.len(),).entered();
        let mut evicted: u32 = 0;

        // ── Phase 1: Per-candidate filtering and eviction loop ────────────────────
        for v_id in candidates {
            if let Some(limit) = stop_at {
                if evicted as usize >= limit {
                    break;
                }
            }

            if !self.is_eviction_candidate(v_id) {
                continue;
            }

            self.evict_tip(v_id);
            evicted += 1;

            if tracing::enabled!(tracing::Level::DEBUG) {
                crate::diagnostics::diagnostic::audit_violations(
                    &self.core.vtree.nodes,
                    &self.core.vtree.violations,
                    "POST-EVICT",
                );
            }

            if cfg!(debug_assertions) {
                self.debug_assert_plateau_mirror_consistency(&format!(
                    "POST-EVICT-SINGLE-{evicted}"
                ));
            }
        }

        // ── Phase 2: Post-batch rebalance, plateau repair, and normalisation ───
        if evicted > 0 {
            let new_gnodes = self.core.rebalance();
            if !new_gnodes.is_empty() {
                self.handle_legacy_promotes(&new_gnodes);
            }

            self.repair_p_i4();

            self.tracker.set_dirty();
            self.normalize_plateaus();

            self.repair_p_i4();
        }

        if cfg!(debug_assertions) || tracing::enabled!(tracing::Level::DEBUG) {
            self.debug_assert_plateau_mirror_consistency("POST-EVICT");
        }

        evicted
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::handle::VNodeId;
    use crate::tree::vtree::vnode::VKind;

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

    /// Config that allows depth-gate tightening.
    ///
    /// `depth_buffer` = 2, headroom = 3^3 = 27, `soft_limit` = 30 - 27 = 3.
    /// After the second split `node_count` = 5 > `soft_limit` = 3, so
    /// `adjust_depth_gates` must tighten `live_depth_evict` from 4 → 3.
    fn gate_config() -> Config<u32> {
        Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 2,
                depth_evict: 4,
                budget: Some(30),
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        }
    }

    // ── check_evictions ───────────────────────────────────────────────
    mod check_evictions_fn {
        use super::*;

        #[test]
        fn returns_zero_for_fresh_graph() {
            let mut g: G = GvGraph::new(make_config());
            assert_eq!(g.check_evictions(), 0);
        }

        #[test]
        fn returns_zero_when_all_nodes_are_shallow() {
            let mut g: G = GvGraph::new(make_config());
            g.observe(64u8, 3u32); // bootstrap split — leaves at depth 2, evict=5
            assert_eq!(g.check_evictions(), 0);
        }
    }

    // ── adjust_depth_gates ────────────────────────────────────────────
    mod adjust_depth_gates_fn {
        use super::*;

        #[test]
        fn no_budget_leaves_gates_unchanged() {
            let mut g: G = GvGraph::new(make_config());
            let d0 = g.depth_evict();
            g.observe(64u8, 3u32);
            assert_eq!(g.depth_evict(), d0, "gates must not change without budget");
        }

        #[test]
        fn tightens_when_node_count_exceeds_soft_limit() {
            // soft_limit=3; bootstrap gives count=3 (not > 3).
            // Catalytic split raises count to 5 > 3 → tighten live_depth_evict.
            let mut g: G = GvGraph::new(gate_config());
            let d0 = g.depth_evict(); // starts at 4
            // bootstrap split (count→3, soft_limit=3, 3 > 3 is false → no tighten yet)
            g.observe(64u8, 3u32);
            assert_eq!(g.depth_evict(), d0);
            // catalytic split: count→5 > 3 → tighten
            g.observe(32u8, 3u32);
            assert!(g.depth_evict() < d0, "live_depth_evict must have tightened");
        }

        #[test]
        fn relaxes_when_count_is_below_threshold() {
            // With gate_config: soft_limit=3, alpha_relax=0.5, threshold=1.5.
            // Fresh graph has count=1; observing with delta=1 (< split_threshold=2)
            // keeps count=1 < threshold=1.5 → relax: live_depth_evict 4 → 5.
            let mut g: G = GvGraph::new(gate_config());
            let d0 = g.depth_evict(); // starts at 4
            g.observe(64u8, 1u32); // no split, count stays at 1
            assert!(g.depth_evict() > d0, "live_depth_evict must have relaxed");
        }
    }

    // ── check_evictions_bounded ───────────────────────────────────────
    mod check_evictions_bounded_fn {
        use super::*;

        #[test]
        fn bounded_returns_zero_when_no_deep_candidates() {
            let mut g: G = GvGraph::new(make_config());
            g.observe(64u8, 3u32); // bootstrap split — leaves at depth 2, evict=5
            assert_eq!(g.check_evictions_bounded(1), 0);
        }
    }

    // ── is_eviction_candidate ─────────────────────────────────────────
    mod is_eviction_candidate_fn {
        use super::*;

        fn find_non_root_entry(g: &G) -> VNodeId {
            let mut best: Option<(VNodeId, u32)> = None;
            for (idx, n) in g.vnodes().iter_occupied() {
                if let VKind::Entry { gnode, .. } = n.kind() {
                    if *gnode != g.g_root() {
                        let id = VNodeId::from_index(idx);
                        let depth = g.core.vtree.depth(id);
                        if best.is_none_or(|(_, d)| depth > d) {
                            best = Some((id, depth));
                        }
                    }
                }
            }
            best.map(|(id, _)| id)
                .expect("expected at least one non-root entry")
        }

        fn low_evict_config() -> Config<u32> {
            Config {
                split_threshold: 2,
                structural: StructuralConfig {
                    depth_create: 1,
                    depth_evict: 2,
                    budget: None,
                    alpha_relax: 0.5,
                    bounded_eviction: false,
                },
            }
        }

        #[test]
        fn returns_false_when_depth_is_not_above_gate() {
            let mut g: G = GvGraph::new(make_config());
            g.observe(64u8, 3u32); // depth ~2 leaves, gate=5
            let id = find_non_root_entry(&g);
            assert!(!g.is_eviction_candidate(id));
        }

        #[test]
        fn returns_true_for_deep_evictable_non_root_entry() {
            let mut g: G = GvGraph::new(low_evict_config());
            g.observe(64u8, 3u32);
            g.observe(32u8, 3u32);
            g.observe(16u8, 3u32); // drive deeper than D_evict=2
            g.core.gtree.live_depth_evict = 0;
            let id = find_non_root_entry(&g);
            if let VKind::Entry { is_evictable, .. } =
                g.core.vtree.nodes.get_mut(id.index()).kind_mut()
            {
                *is_evictable = true;
            }
            assert!(g.is_eviction_candidate(id));
        }

        #[test]
        fn returns_false_for_structural_node() {
            let mut g: G = GvGraph::new(make_config());
            g.observe(64u8, 3u32);
            let root = g.v_root().expect("v_root must exist after split");
            assert!(!g.is_eviction_candidate(root));
        }
    }

    mod handle_legacy_promotes_fn {
        use super::*;

        #[test]
        fn increments_node_and_terminal_counts_per_new_gnode() {
            let mut g: G = GvGraph::new(make_config());
            let root = g.core.gtree.nodes.root;
            let g1 = g.core.gtree.nodes.allocate_missing_child(root);
            let n0 = g.core.gtree.nodes.node_count;
            let t0 = g.core.gtree.nodes.terminal_count;

            let new_gnodes = [g1];
            g.handle_legacy_promotes(&new_gnodes);

            assert_eq!(g.core.gtree.nodes.node_count, n0 + 1);
            assert_eq!(g.core.gtree.nodes.terminal_count, t0 + 1);
        }
    }
}
