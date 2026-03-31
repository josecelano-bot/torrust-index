//! Incremental plateau-mirror tracker for `dynamic-contour-tracking` builds.
//!
//! [`DynamicPlateauTracker`] owns all plateau-related state and all plateau
//! algorithms that were previously scattered across `GvGraph` as
//! `#[cfg(feature = "dynamic-contour-tracking")]` fields and `impl` blocks.
//!
//! It implements the [`PlateauTracking`] strategy trait and is threaded into
//! `GvGraph` as a type parameter (Phase 7 Step 7.3).

#![cfg(feature = "dynamic-contour-tracking")]

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::handle::GNodeId;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::spatial::plateau_basis::PlateauBasis;
use crate::traits::{Accumulator, Coordinate, PlateauTracking};
use crate::tree::gtree::GNodeTree;
use crate::tree::gtree::gnode::GState;

mod core_helpers;
mod debug_diff;
mod debug_sums;
mod evict;
mod legacy_promotes;
mod normalize;
mod observe;
mod repair;
mod split_bootstrap;
mod split_catalytic;

/// Incrementally maintains a mirror of the plateau structure as observations,
/// splits, and evictions mutate the G-tree.
///
/// All heavyweight plateau logic previously lived on `GvGraph` as `impl` blocks
/// in `plateau/mod.rs` and `plateau/normalise.rs`.  This struct now owns both
/// the *state* and the *algorithms* for plateau tracking.
#[derive(Debug, Clone)]
pub struct DynamicPlateauTracker<C: Coordinate, V: Accumulator> {
    /// The plateau mirror: maps each basis edge to its current plateau.
    pub(crate) plateaus: BTreeMap<BasisEdge<C>, Plateau<C, V>>,

    /// Pending (semi-internal node, plateau key) pairs that must be checked by
    /// the P-I4 repair pass.
    pub(crate) pending_p_i4: Vec<(GNodeId, BasisEdge<C>)>,

    /// Reverse index: maps each G-node id to the basis-edge key of the plateau
    /// it contributes to.
    pub(crate) plateau_basis: PlateauBasis<C>,

    /// True when at least one mutating operation has occurred since the last
    /// `normalize` call.
    pub(crate) plateaus_dirty: bool,

    /// The coordinate bit-width (`N` from `GvGraph<C, V, N>`), stored at
    /// runtime so depth computations do not require the const generic here.
    pub(crate) n_bits: u32,
}

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    /// Creates a tracker pre-populated with the single root plateau.
    #[must_use]
    pub(crate) fn with_root(
        root_key: BasisEdge<C>,
        root_depth: u32,
        root_lo: C,
        root_hi: C,
        g_root: GNodeId,
        n_bits: u32,
    ) -> Self {
        let mut pb = PlateauBasis::new();
        pb.insert(root_key, g_root);
        let mut map = BTreeMap::new();
        map.insert(
            root_key,
            Plateau {
                basis_edge: root_key,
                start: root_lo,
                end: root_hi,
                depth: root_depth,
                sum: V::zero(),
            },
        );
        Self {
            plateaus: map,
            pending_p_i4: Vec::new(),
            plateau_basis: pb,
            plateaus_dirty: false,
            n_bits,
        }
    }
}

// ── PlateauTracking impl ─────────────────────────────────────────────────────

impl<C: Coordinate, V: Accumulator> PlateauTracking<C, V> for DynamicPlateauTracker<C, V> {
    fn on_observe(&mut self, gnodes: &GNodeTree<C, V>, g_id: GNodeId, value: V) {
        self.on_observe_impl(gnodes, g_id, value);
    }

    fn on_bootstrap_split(&mut self, gnodes: &GNodeTree<C, V>, g_id: GNodeId, _left_id: GNodeId) {
        self.on_bootstrap_split_impl(gnodes, g_id);
    }

    fn on_catalytic_split(&mut self, gnodes: &GNodeTree<C, V>, g_id: GNodeId, left_id: GNodeId) {
        self.on_catalytic_split_impl(gnodes, g_id, left_id);
    }

    fn on_evict(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        gnode_id: GNodeId,
        parent_id: GNodeId,
        parent_state_after: GState,
        parent_lo: C,
        parent_hi: C,
    ) {
        self.on_evict_impl(
            gnodes,
            gnode_id,
            parent_id,
            parent_state_after,
            parent_lo,
            parent_hi,
        );
    }

    fn on_legacy_promotes_batched(&mut self, gnodes: &GNodeTree<C, V>, new_gnodes: &[GNodeId]) {
        self.on_legacy_promotes_batched_impl(gnodes, new_gnodes);
    }

    #[allow(clippy::too_many_lines, clippy::float_cmp)]
    fn normalize(&mut self, gnodes: &GNodeTree<C, V>) {
        self.normalize_impl(gnodes);
    }

    fn repair_p_i4(&mut self, gnodes: &GNodeTree<C, V>) {
        self.repair_p_i4_impl(gnodes);
    }

    fn recompute_sums(&mut self, gnodes: &GNodeTree<C, V>, label: &str) {
        #[cfg(not(debug_assertions))]
        let _ = label;

        for (&key, plateau) in &mut self.plateaus {
            plateau.sum = self
                .plateau_basis
                .basis_elements(&key)
                .iter()
                .map(|&r| gnodes.get(r.index()).sum())
                .fold(V::zero(), V::add);
        }
        #[cfg(debug_assertions)]
        for (&key, plateau) in &self.plateaus {
            let expected: V = self
                .plateau_basis
                .basis_elements(&key)
                .iter()
                .map(|&r| gnodes.get(r.index()).sum())
                .fold(V::zero(), V::add);
            assert_eq!(
                plateau.sum, expected,
                "POST-{label}: plateau sum drift at key {key:?}"
            );
        }
    }

    fn set_dirty(&mut self) {
        self.plateaus_dirty = true;
    }

    fn plateaus(&self) -> Cow<'_, BTreeMap<BasisEdge<C>, Plateau<C, V>>> {
        Cow::Borrowed(&self.plateaus)
    }

    fn debug_assert_mirror_consistency(
        &self,
        gnodes: &GNodeTree<C, V>,
        fresh: &BTreeMap<BasisEdge<C>, Plateau<C, V>>,
        label: &str,
    ) {
        self.debug_assert_mirror_consistency_impl(gnodes, fresh, label);
    }
}

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
mod tests {
    include!("dynamic_tracker/tests.rs");
}
