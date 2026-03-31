use super::DynamicPlateauTracker;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GNodeTree;
use crate::tree::gtree::gnode::GState;
use crate::tree::gtree::gnode_depth_from_range;
use crate::tree::handle::GNodeId;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(super) fn on_legacy_promotes_batched_impl(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        new_gnodes: &[GNodeId],
    ) {
        if new_gnodes.is_empty() {
            return;
        }

        let _span = tracing::debug_span!(
            "plateau_after_legacy_promotes_batched",
            count = new_gnodes.len(),
        )
        .entered();

        self.plateaus_dirty = true;

        let mut to_place = Vec::new();

        for &new_gid in new_gnodes {
            let ng = gnodes.get(new_gid.index());
            let parent_id = ng
                .parent()
                .expect("legacy_promote child must have a parent");
            let child_depth = gnode_depth_from_range(ng.range(), self.n_bits);

            let pg = gnodes.get(parent_id.index());
            let existing_child_id = if pg.left() == Some(new_gid) {
                pg.right()
            } else {
                pg.left()
            };

            if let Some(old_key) = self.plateau_basis.remove(parent_id) {
                self.fixup_plateau(gnodes, old_key);
            }

            if let Some(ec_id) = existing_child_id {
                let ec = gnodes.get(ec_id.index());
                if ec.state() != GState::Internal && self.plateau_basis.plateau_key(ec_id).is_none()
                {
                    let existing_depth = gnode_depth_from_range(ec.range(), self.n_bits);
                    to_place.push((ec_id, existing_depth));
                }
            }
            to_place.push((new_gid, child_depth));
        }

        self.place_sorted(gnodes, &mut to_place);
    }
}
