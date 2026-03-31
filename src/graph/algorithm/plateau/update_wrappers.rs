use crate::graph::GvGraph;
use crate::handle::GNodeId;
use crate::traits::{Accumulator, Coordinate, Inspectable, PlateauTracking};

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32, T: PlateauTracking<C, V>>
    GvGraph<C, V, N, T>
{
    pub(crate) fn plateau_after_observe<O: crate::traits::Observation<V>>(
        &mut self,
        g_id: GNodeId,
        delta: O,
    ) {
        let value_v: V = O::accumulate(V::zero(), delta);
        self.tracker
            .on_observe(&self.core.gtree.nodes, g_id, value_v);
    }

    pub(crate) fn plateau_after_bootstrap_split(&mut self, g_id: GNodeId, left_id: GNodeId) {
        self.tracker
            .on_bootstrap_split(&self.core.gtree.nodes, g_id, left_id);
    }

    pub(crate) fn plateau_after_catalytic_split(&mut self, g_id: GNodeId, left_id: GNodeId) {
        self.tracker
            .on_catalytic_split(&self.core.gtree.nodes, g_id, left_id);
    }

    pub(crate) fn plateau_after_legacy_promotes_batched(&mut self, new_gnodes: &[GNodeId]) {
        self.tracker
            .on_legacy_promotes_batched(&self.core.gtree.nodes, new_gnodes);
    }

    pub(crate) fn plateau_after_evict(
        &mut self,
        gnode_id: GNodeId,
        parent_id: GNodeId,
        parent_state_after: crate::nodes::gnode::GState,
        parent_lo: C,
        parent_hi: C,
    ) {
        self.tracker.on_evict(
            &self.core.gtree.nodes,
            gnode_id,
            parent_id,
            parent_state_after,
            parent_lo,
            parent_hi,
        );
    }

    pub(crate) fn normalize_plateaus(&mut self) {
        self.tracker.normalize(&self.core.gtree.nodes);
    }

    pub(crate) fn repair_p_i4(&mut self) {
        self.tracker.repair_p_i4(&self.core.gtree.nodes);
    }

    pub(crate) fn plateau_recompute_sums(&mut self, label: &str) {
        self.tracker.recompute_sums(&self.core.gtree.nodes, label);
    }

    /// Verifies that the plateau mirror is consistent with a fresh G-tree
    /// rebuild. A no-op with `NoopPlateauTracker`; panics on divergence with
    /// `DynamicPlateauTracker`. Called at algorithmic debug checkpoints.
    pub(crate) fn debug_assert_plateau_mirror_consistency(&self, label: &str) {
        if !cfg!(debug_assertions) && !tracing::enabled!(tracing::Level::DEBUG) {
            return;
        }
        let fresh = self.build_plateaus();
        self.tracker
            .debug_assert_mirror_consistency(&self.core.gtree.nodes, &fresh, label);
    }
}
