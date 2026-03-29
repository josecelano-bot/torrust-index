use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::{GNode, GState};
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::gnode_depth_from_interval;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(super) fn on_evict_impl(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        gnode_id: GNodeId,
        parent_id: GNodeId,
        parent_state_after: GState,
        parent_lo: C,
        parent_hi: C,
    ) {
        let _span = tracing::debug_span!(
            "plateau_after_evict",
            gnode = gnode_id.index(),
            parent = parent_id.index(),
            ?parent_state_after,
        )
        .entered();

        // ── Phase 1: Remove evicted node and parent from plateau basis ────────────
        let evicted_key = self.plateau_basis.remove(gnode_id);

        let mut displaced: Vec<GNodeId> = Vec::new();
        let mut displaced_extra: Vec<(GNodeId, u32)> = Vec::new();

        let ancestor_key = if let Some(key) = self.plateau_basis.remove(parent_id) {
            if parent_state_after == GState::SemiInternal {
                let g = gnodes.get(parent_id.index());
                if let Some(sib) = g.left().or_else(|| g.right()) {
                    if let Some(ok) = self.plateau_basis.remove(sib) {
                        self.fixup_plateau(gnodes, ok);
                    }
                    displaced.push(sib);
                }
            }
            Some(key)
        } else {
            self.evict_ancestor_key(gnodes, parent_id, parent_state_after, &mut displaced)
        };

        // ── Phase 2: Fixup displaced plateau keys ────────────────────────────────
        if let Some(ek) = evicted_key {
            self.fixup_plateau(gnodes, ek);
        }
        if let Some(ak) = ancestor_key {
            if evicted_key != Some(ak) {
                self.fixup_plateau(gnodes, ak);
            }
        }

        // ── Phase 3: Compute parent depth after eviction ─────────────────────────
        let parent_depth = match parent_state_after {
            GState::Terminal | GState::SemiInternal => {
                gnode_depth_from_interval(parent_lo, parent_hi, self.n_bits)
            }
            GState::Internal => {
                unreachable!("evict_tip: parent cannot remain Internal after eviction")
            }
        };

        // ── Phases 4+5: Evacuate adjacent same-depth plateaus ────────────────────
        self.evacuate_adjacent_plateaus(
            gnodes,
            parent_lo,
            parent_hi,
            parent_depth,
            &mut displaced_extra,
        );

        // ── Phase 6: Collect displaced nodes and re-place ────────────────────────
        let mut to_place = Vec::new();
        to_place.push((parent_id, parent_depth));
        for &sib_id in &displaced {
            self.collect_subtree_basis_elements(gnodes, sib_id, &mut to_place);
        }
        to_place.extend(displaced_extra);
        self.place_sorted(gnodes, &mut to_place);
    }
}
