use crate::graph::GvGraph;
use crate::traits::{Accumulator, Coordinate, Weighable};
use crate::tree::gtree::GTree;
use crate::tree::handle::VNodeId;

impl<C: Coordinate, V: Accumulator + Weighable, const N: u32> GvGraph<C, V, N> {
    #[must_use]
    #[allow(clippy::doc_markdown)]
    pub fn sample(
        &self,
        rng: &mut impl crate::traits::Rng,
    ) -> Option<crate::spatial::view::Cell<C, V>> {
        use crate::tree::vtree::vnode::VKind;

        let v_root = self.core.vtree.nodes.root?;
        let root_node = self.core.vtree.nodes.get(v_root.index());
        if root_node.intensity() == V::zero() {
            return None;
        }

        let mut current = v_root;
        loop {
            let vnode = self.core.vtree.nodes.get(current.index());
            match &vnode.kind() {
                VKind::Entry { gnode, .. } => {
                    let g = self.core.gtree.nodes.get(gnode.index());
                    let (start, end) = Self::uncovered_interval(g);
                    return Some(crate::spatial::view::Cell {
                        start,
                        end,
                        intensity: g.own(),
                        depth: GTree::<C, V, N>::depth_of_interval(start, end),
                    });
                }
                VKind::Structural { children, .. } => {
                    current = Self::sample_child(children, rng);
                }
            }
        }
    }

    fn sample_child(
        children: &crate::tree::vtree::vnode::Children<V>,
        rng: &mut impl crate::traits::Rng,
    ) -> VNodeId {
        let total: f64 = (0..children.len())
            .map(|i| children.get(i).1.weight())
            .sum();
        debug_assert!(total > 0.0, "sample_child: zero-total children");

        let threshold = rng.next_f64() * total;
        let mut cumulative = 0.0_f64;
        for i in 0..children.len() {
            cumulative += children.get(i).1.weight();
            if threshold < cumulative {
                return children.get(i).0;
            }
        }

        children.get(children.len() - 1).0
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::traits::Rng;

    type G = GvGraph<u8, u32, 8>;

    const fn make_config() -> Config<u32> {
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

    struct FixedRng(f64);
    impl Rng for FixedRng {
        fn next_f64(&mut self) -> f64 {
            self.0
        }
    }

    #[test]
    fn returns_none_for_zero_sum_graph() {
        let g = fresh_graph();
        assert!(g.sample(&mut FixedRng(0.5)).is_none());
    }

    #[test]
    fn returns_some_after_at_least_one_observation() {
        let mut g = fresh_graph();
        g.observe(0u8, 10u32);
        assert!(g.sample(&mut FixedRng(0.5)).is_some());
    }

    #[test]
    fn sample_on_split_graph_traverses_structural_vtree() {
        // After enough observations to trigger splits, the vtree contains
        // Structural nodes; sample() must traverse them via sample_child.
        let mut g = fresh_graph();
        for _ in 0..3 {
            g.observe(64u8, 5u32); // bootstrap + further splits
        }
        // The graph has observations so sample returns Some
        let result = g.sample(&mut FixedRng(0.5));
        assert!(result.is_some());
    }

    #[test]
    fn sample_with_rng_near_one_returns_a_cell() {
        // rng near 1.0 exercises paths toward the last child in sample_child
        let mut g = fresh_graph();
        for _ in 0..3 {
            g.observe(64u8, 5u32);
        }
        let result = g.sample(&mut FixedRng(0.999));
        assert!(result.is_some());
    }
}
