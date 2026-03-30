use crate::graph::GvGraph;
use crate::spatial::plateau::BasisEdge;
use crate::traits::{Accumulator, Coordinate, Inspectable};
use crate::tree::gtree::GTree;

#[cfg(feature = "dynamic-contour-tracking")]
use super::dynamic_tracker::DynamicPlateauTracker;

#[cfg(feature = "dynamic-contour-tracking")]
impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32>
    GvGraph<C, V, N, DynamicPlateauTracker<C, V>>
{
    #[must_use]
    #[inline]
    pub(crate) const fn plateau_basis(&self) -> &crate::spatial::plateau_basis::PlateauBasis<C> {
        &self.tracker.plateau_basis
    }

    #[allow(clippy::float_cmp)]
    pub(crate) fn debug_check_plateau_sums(&self, label: &str) {
        self.tracker.debug_check_sums(&self.gtree.nodes, label);
    }

    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::type_complexity)]
    pub fn debug_plateau_basis(&self) -> Vec<(BasisEdge<C>, Vec<(usize, C, C, &'static str, u32)>)> {
        let mut result = Vec::new();
        for &key in self.tracker.plateaus.keys() {
            let elements = self.tracker.plateau_basis.basis_elements(&key);
            let infos: Vec<_> = elements
                .iter()
                .map(|&gid| {
                    let g = self.gtree.nodes.get(gid.index());
                    let state_str = match g.state() {
                        crate::nodes::gnode::GState::Terminal => "Terminal",
                        crate::nodes::gnode::GState::Internal => "Internal",
                        crate::nodes::gnode::GState::SemiInternal => "SemiInternal",
                    };
                    let g_depth = GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi());
                    (gid.index(), g.lo(), g.hi(), state_str, g_depth)
                })
                .collect();
            result.push((key, infos));
        }
        result
    }
}

#[cfg(test)]
#[cfg(feature = "dynamic-contour-tracking")]
mod tests {
    use super::*;
    use crate::graph::{Config, StructuralConfig};

    type G = GvGraph<u8, u32, 8>;

    fn make_graph() -> G {
        GvGraph::new(Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 3,
                depth_evict: 5,
                budget: None,
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        })
    }

    #[test]
    fn debug_plateau_basis_reports_entries_after_observe() {
        let mut g = make_graph();
        g.observe(64u8, 3u32);
        g.observe(32u8, 3u32);

        let rows = g.debug_plateau_basis();
        assert!(!rows.is_empty());

        for (_key, infos) in &rows {
            assert!(!infos.is_empty());
            for &(idx, lo, hi, state, depth) in infos {
                assert!(idx < g.node_count() as usize);
                assert!(lo < hi);
                assert!(matches!(state, "Terminal" | "Internal" | "SemiInternal"));
                assert!(depth > 0);
            }
        }
    }

    #[test]
    fn plateau_basis_accessor_matches_debug_view_count() {
        let mut g = make_graph();
        g.observe(64u8, 3u32);
        g.observe(32u8, 3u32);

        let debug_rows = g.debug_plateau_basis();
        assert_eq!(g.plateau_basis().plateau_count(), debug_rows.len());
    }

    #[test]
    fn debug_check_plateau_sums_does_not_panic_on_valid_graph() {
        let mut g = make_graph();
        g.observe(64u8, 3u32);
        g.observe(128u8, 3u32);
        g.debug_check_plateau_sums("debug-api-test");
    }
}
