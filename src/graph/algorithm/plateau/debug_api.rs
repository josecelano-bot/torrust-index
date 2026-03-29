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
