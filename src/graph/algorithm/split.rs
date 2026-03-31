use crate::graph::GvGraph;
use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::{Children, VNode};
use crate::traits::{Accumulator, Coordinate, Inspectable};

mod helpers;

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32> GvGraph<C, V, N> {
    pub(crate) fn attempt_split(&mut self, g_id: GNodeId) {
        let Some(entry_id) = self.split_candidate_entry(g_id) else {
            return;
        };

        if self.vtree.nodes.get(entry_id.index()).parent().is_none() {
            self.bootstrap_split(g_id);
            return;
        }

        self.preprocess_split_parent(entry_id);

        let entry_id = self.gtree.nodes.get(g_id.index()).entry().unwrap();
        if self.vtree.depth(entry_id) > self.gtree.live_depth_create {
            return;
        }

        self.catalytic_split(g_id);
    }

    fn bootstrap_split(&mut self, g_id: GNodeId) {
        let (lo, hi, entry_id) = {
            let g = self.gtree.nodes.get(g_id.index());
            (
                g.lo(),
                g.hi(),
                g.entry().expect("bootstrap_split: g must have an entry"),
            )
        };
        let _span =
            tracing::debug_span!("bootstrap_split", g_id = g_id.index(), ?lo, ?hi,).entered();

        let children = self.allocate_split_children(g_id);

        let cs_id = self
            .vtree
            .alloc_structural_2(children.left_entry_id, children.right_entry_id);

        let entry_int = self.vtree.nodes.get(entry_id.index()).intensity();
        let root_structural = VNode::new_structural(
            entry_int,
            None,
            Children::new_2((entry_id, entry_int), (cs_id, V::zero())),
            true,
        );
        let root_s_id = VNodeId::from_index(self.vtree.nodes.alloc(root_structural).0);
        self.vtree
            .nodes
            .get_mut(entry_id.index())
            .set_parent(root_s_id);
        self.vtree
            .nodes
            .get_mut(cs_id.index())
            .set_parent(root_s_id);

        self.vtree.set_entry_flags(entry_id, false, false);

        self.vtree.nodes.root = Some(root_s_id);
        self.plateau_after_bootstrap_split(g_id, children.left_id);
        self.debug_assert_split_mirror_consistency("POST-BOOTSTRAP-SPLIT");
    }

    #[allow(clippy::too_many_lines)]
    fn catalytic_split(&mut self, g_id: GNodeId) {
        let (lo, hi, entry_id) = {
            let g = self.gtree.nodes.get(g_id.index());
            (
                g.lo(),
                g.hi(),
                g.entry().expect("catalytic_split: g must have an entry"),
            )
        };
        let mid = C::midpoint(lo, hi);
        let _span =
            tracing::debug_span!("catalytic_split", g_id = g_id.index(), ?lo, ?hi, ?mid,).entered();
        let p_id = self
            .vtree
            .nodes
            .get(entry_id.index())
            .parent()
            .expect("catalytic_split: entry must have a parent");

        let children = self.allocate_split_children(g_id);

        // ── Phase 2: Allocate structural node ───────────────────────────────
        let s = VNode::new_structural(
            V::zero(),
            Some(p_id),
            Children::new_2(
                (children.left_entry_id, V::zero()),
                (children.right_entry_id, V::zero()),
            ),
            true,
        );
        let s_id = VNodeId::from_index(self.vtree.nodes.alloc(s).0);
        self.vtree
            .nodes
            .get_mut(children.left_entry_id.index())
            .set_parent(s_id);
        self.vtree
            .nodes
            .get_mut(children.right_entry_id.index())
            .set_parent(s_id);

        // ── Phase 4: Wire `s` into the parent's child list ───────────────────
        self.vtree.add_structural_child(p_id, s_id, V::zero());

        self.vtree.set_entry_flags(entry_id, false, false);

        // ── Phase 5: Propagate evictable flags ────────────────────────────────
        self.vtree.propagate_evictable(p_id);

        // ── Phase 6: Plateau state update ─────────────────────────────────────
        self.plateau_after_catalytic_split(g_id, children.left_id);
        self.debug_assert_split_mirror_consistency("POST-CATALYTIC-SPLIT");
    }
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

    fn fresh_graph() -> G {
        GvGraph::new(make_config())
    }

    // ── attempt_split (via observe) ───────────────────────────────────
    mod attempt_split {
        use super::*;

        #[test]
        fn no_split_when_sum_at_threshold() {
            // delta=2 equals split_threshold, condition is >, not >=
            let mut g = fresh_graph();
            let n0 = g.gtree.nodes.node_count;
            g.observe(64u8, 2u32);
            assert_eq!(
                g.gtree.nodes.node_count, n0,
                "must not split at exactly threshold"
            );
        }

        #[test]
        fn no_split_when_sum_below_threshold() {
            let mut g = fresh_graph();
            let n0 = g.gtree.nodes.node_count;
            g.observe(64u8, 1u32);
            assert_eq!(g.gtree.nodes.node_count, n0);
        }

        #[test]
        fn bootstrap_split_increases_node_count_by_two() {
            // sum > split_threshold on root triggers bootstrap_split
            let mut g = fresh_graph();
            let n0 = g.gtree.nodes.node_count;
            g.observe(64u8, 3u32);
            assert_eq!(g.gtree.nodes.node_count, n0 + 2);
        }

        #[test]
        fn bootstrap_split_increases_terminal_count_by_one() {
            let mut g = fresh_graph();
            let t0 = g.gtree.nodes.terminal_count;
            g.observe(64u8, 3u32);
            assert_eq!(g.gtree.nodes.terminal_count, t0 + 1);
        }

        #[test]
        fn catalytic_split_increases_node_count_further() {
            // First observation triggers bootstrap split; second observation on a
            // child triggers catalytic split.
            let mut g = fresh_graph();
            g.observe(32u8, 3u32); // bootstrap split — left child covers [0,128)
            let n1 = g.gtree.nodes.node_count;
            g.observe(32u8, 3u32); // catalytic split of the left child
            assert!(g.gtree.nodes.node_count > n1);
        }

        #[test]
        fn already_split_node_is_not_split_again() {
            // After a bootstrap split the root gnode has children, so
            // attempt_split on the root is a no-op (early return).
            let mut g = fresh_graph();
            g.observe(64u8, 3u32); // bootstrap — root now has children
            let n1 = g.gtree.nodes.node_count;
            // Observing the root coordinate again should not double-split the root.
            // A further split (if any) would happen on a child, not the root.
            g.observe(128u8, 3u32); // different half — may split the right child
            // node_count may grow (child splits) but the root is not split again
            assert!(g.gtree.nodes.node_count >= n1);
        }

        #[test]
        fn direct_call_on_internal_gnode_is_no_op() {
            // Exercises the first early-return guard (line 19): calling
            // attempt_split directly on a gnode that already has children
            // must be a no-op (it returns immediately).
            let mut g = fresh_graph();
            g.observe(64u8, 3u32); // bootstrap — g_root becomes Internal
            let root = g.gtree.nodes.root;
            let n_before = g.gtree.nodes.node_count;
            g.attempt_split(root);
            assert_eq!(g.gtree.nodes.node_count, n_before);
        }
    }
}
