use super::DynamicPlateauTracker;
use crate::tree::gtree::GNodeTree;
use crate::handle::GNodeId;
use crate::nodes::gnode::GState;
use crate::spatial::plateau::BasisEdge;
use crate::traits::{Accumulator, Coordinate};

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(super) fn repair_p_i4_impl(&mut self, gnodes: &GNodeTree<C, V>) {
        let span = tracing::debug_span!(
            "repair_p_i4",
            pending = self.pending_p_i4.len(),
            candidates = tracing::field::Empty,
        )
        .entered();

        let mut candidates: Vec<(GNodeId, BasisEdge<C>)> = Vec::new();
        for (&key, elements) in self.plateau_basis.iter() {
            for &gid in elements {
                if gnodes.is_occupied(gid.index())
                    && gnodes.get(gid.index()).state() == GState::SemiInternal
                {
                    candidates.push((gid, key));
                }
            }
        }
        self.pending_p_i4.extend(candidates);
        span.record("candidates", self.pending_p_i4.len());

        while let Some((gid, pk)) = self.pending_p_i4.pop() {
            if self.plateau_basis.plateau_key(gid) != Some(pk) {
                continue;
            }
            if !gnodes.is_occupied(gid.index()) {
                continue;
            }
            let g = gnodes.get(gid.index());
            if g.state() != GState::SemiInternal {
                continue;
            }

            let Some(child_id) = g.left().or_else(|| g.right()) else {
                continue;
            };
            let child_lo = gnodes.get(child_id.index()).lo();
            let child_pk = self
                .plateaus
                .range(..=BasisEdge(child_lo))
                .next_back()
                .map(|(&k, _)| k);

            if child_pk == Some(pk) {
                self.split_for_p_i4(gnodes, pk, child_id);
            }
        }
    }
}
