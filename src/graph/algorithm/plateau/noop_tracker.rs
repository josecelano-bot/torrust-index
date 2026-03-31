//! Zero-cost no-op plateau tracker for builds without `dynamic-contour-tracking`.

use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::tree::gtree::GNodeTree;
use crate::handle::GNodeId;
use crate::nodes::gnode::GState;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::{Accumulator, Coordinate, PlateauTracking};

/// Zero-cost placeholder that satisfies the [`PlateauTracking`] bound while
/// performing no work.  Selected when `dynamic-contour-tracking` is disabled.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPlateauTracker;

impl<C: Coordinate, V: Accumulator> PlateauTracking<C, V> for NoopPlateauTracker {
    #[inline(always)]
    fn on_observe(&mut self, _gnodes: &GNodeTree<C, V>, _g_id: GNodeId, _value: V) {}

    #[inline(always)]
    fn on_bootstrap_split(
        &mut self,
        _gnodes: &GNodeTree<C, V>,
        _g_id: GNodeId,
        _left_id: GNodeId,
    ) {
    }

    #[inline(always)]
    fn on_catalytic_split(
        &mut self,
        _gnodes: &GNodeTree<C, V>,
        _g_id: GNodeId,
        _left_id: GNodeId,
    ) {
    }

    #[inline(always)]
    fn on_evict(
        &mut self,
        _gnodes: &GNodeTree<C, V>,
        _gnode_id: GNodeId,
        _parent_id: GNodeId,
        _parent_state_after: GState,
        _parent_lo: C,
        _parent_hi: C,
    ) {
    }

    #[inline(always)]
    fn on_legacy_promotes_batched(
        &mut self,
        _gnodes: &GNodeTree<C, V>,
        _new_gnodes: &[GNodeId],
    ) {
    }

    #[inline(always)]
    fn normalize(&mut self, _gnodes: &GNodeTree<C, V>) {}

    #[inline(always)]
    fn repair_p_i4(&mut self, _gnodes: &GNodeTree<C, V>) {}

    #[inline(always)]
    fn recompute_sums(&mut self, _gnodes: &GNodeTree<C, V>, _label: &str) {}

    #[inline(always)]
    fn set_dirty(&mut self) {}

    #[inline(always)]
    fn plateaus(&self) -> Cow<'_, BTreeMap<BasisEdge<C>, Plateau<C, V>>> {
        Cow::Owned(BTreeMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::nodes::gnode::GNode;
    use crate::tree::gtree::GNodeTree;

    fn empty_gnodes() -> GNodeTree<u8, u32> {
        let mut nodes: Arena<GNode<u8, u32>> = Arena::new();
        let root_idx = nodes.alloc(GNode::new_leaf(0u8, 255u8, 0u32, None)).0;
        GNodeTree {
            nodes,
            root: GNodeId::from_index(root_idx),
            node_count: 1,
            terminal_count: 1,
        }
    }

    fn exercise_tracker(tracker: &mut NoopPlateauTracker, gnodes: &GNodeTree<u8, u32>) {
        let root = gnodes.root;
        tracker.on_observe(gnodes, root, 0u32);
        tracker.on_bootstrap_split(gnodes, root, root);
        tracker.on_catalytic_split(gnodes, root, root);
        tracker.on_evict(
            gnodes,
            root,
            root,
            crate::nodes::gnode::GState::Terminal,
            0u8,
            255u8,
        );
        tracker.on_legacy_promotes_batched(gnodes, &[]);
        tracker.normalize(gnodes);
        tracker.repair_p_i4(gnodes);
        tracker.recompute_sums(gnodes, "test");
        <NoopPlateauTracker as PlateauTracking<u8, u32>>::set_dirty(tracker);
        let p = <NoopPlateauTracker as PlateauTracking<u8, u32>>::plateaus(tracker);
        assert!(p.is_empty(), "NoopPlateauTracker must return empty plateau map");
    }

    #[test]
    fn all_methods_run_without_panicking() {
        let gnodes = empty_gnodes();
        let mut tracker = NoopPlateauTracker;
        exercise_tracker(&mut tracker, &gnodes);
    }

    #[test]
    fn debug_assert_mirror_consistency_default_does_not_panic() {
        use crate::spatial::plateau::{BasisEdge, Plateau};
        use std::collections::BTreeMap;

        let gnodes = empty_gnodes();
        let tracker = NoopPlateauTracker;
        let fresh: BTreeMap<BasisEdge<u8>, Plateau<u8, u32>> = BTreeMap::new();

        // The default impl is a no-op; calling it must not panic.
        <NoopPlateauTracker as PlateauTracking<u8, u32>>::debug_assert_mirror_consistency(
            &tracker, &gnodes, &fresh, "test",
        );
    }
}
