pub use super::fmt::{Ctx, Nd};

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

    fn fresh() -> G {
        GvGraph::new(make_config())
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
        use crate::tree::handle::VNodeId;

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
        use crate::tree::vtree::vnode::VKind;

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
        use crate::graph::algorithm::fmt::Ch;

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
}
