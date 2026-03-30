use super::DynamicPlateauTracker;
use crate::tree::gtree::GNodeTree;
use crate::handle::GNodeId;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::gnode_depth_from_range;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(super) fn on_bootstrap_split_impl(&mut self, gnodes: &GNodeTree<C, V>, g_id: GNodeId) {
        let _span =
            tracing::debug_span!("plateau_after_bootstrap_split", g_id = g_id.index(),).entered();

        self.plateaus_dirty = true;

        let old_key = self
            .plateau_basis
            .remove(g_id)
            .expect("bootstrap_split: g_id must be a basis element");
        self.fixup_plateau(gnodes, old_key);

        let right_id = gnodes
            .get(g_id.index())
            .right()
            .expect("bootstrap_split: g_id must have a right child");
        let left_id = gnodes
            .get(g_id.index())
            .left()
            .expect("bootstrap_split: g_id must have a left child");

        let left_depth = gnode_depth_from_range(gnodes.get(left_id.index()).range(), self.n_bits);
        let right_depth = gnode_depth_from_range(gnodes.get(right_id.index()).range(), self.n_bits);

        if left_depth == right_depth {
            self.place_basis_element(gnodes, g_id, left_depth);
        } else {
            tracing::debug!(
                g = g_id.index(),
                left = left_id.index(),
                right = right_id.index(),
                left_depth,
                right_depth,
                "bootstrap_split: unequal child depths, placing children separately"
            );
            self.place_basis_element(gnodes, left_id, left_depth);
            self.place_basis_element(gnodes, right_id, right_depth);
        }
    }
}
