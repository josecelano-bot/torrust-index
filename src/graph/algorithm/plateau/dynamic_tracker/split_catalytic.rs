use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::GNode;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::gnode_depth_from_range;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    fn catalytic_split_depths(
        &self,
        gnodes: &Arena<GNode<C, V>>,
        g_id: GNodeId,
        left_id: GNodeId,
    ) -> (GNodeId, u32, u32) {
        let right_id = gnodes
            .get(g_id.index())
            .right()
            .expect("catalytic_split: g_id must have a right child");

        let left_depth = gnode_depth_from_range(gnodes.get(left_id.index()).range(), self.n_bits);
        let right_depth = gnode_depth_from_range(gnodes.get(right_id.index()).range(), self.n_bits);

        (right_id, left_depth, right_depth)
    }

    fn remove_co_members_from_key(&mut self, key: crate::spatial::plateau::BasisEdge<C>) -> Vec<GNodeId> {
        let co_members: Vec<GNodeId> = self
            .plateau_basis
            .basis_elements(&key)
            .iter()
            .copied()
            .collect();
        for &m in &co_members {
            self.plateau_basis.remove(m);
        }
        co_members
    }

    fn collect_path_siblings_for_split(
        &self,
        gnodes: &Arena<GNode<C, V>>,
        path: &[GNodeId],
    ) -> Vec<GNodeId> {
        let mut displaced: Vec<GNodeId> = Vec::new();
        for &path_node in path {
            let par = gnodes
                .get(path_node.index())
                .parent()
                .expect("path node must have a parent");
            let pg = gnodes.get(par.index());
            let sibling = if pg.left() == Some(path_node) {
                pg.right()
            } else {
                pg.left()
            };
            if let Some(sib_id) = sibling {
                displaced.push(sib_id);
            }
        }
        displaced
    }

    fn locate_covering_basis_for_catalytic_split(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        g_id: GNodeId,
    ) -> (crate::spatial::plateau::BasisEdge<C>, Vec<GNodeId>) {
        if let Some(key) = self.plateau_basis.remove(g_id) {
            let displaced = self.remove_co_members_from_key(key);
            return (key, displaced);
        }

        let mut path: Vec<GNodeId> = vec![g_id];
        let mut parent_opt = gnodes.get(g_id.index()).parent();

        while let Some(p_id) = parent_opt {
            if let Some(key) = self.plateau_basis.remove(p_id) {
                let mut displaced = self.collect_path_siblings_for_split(gnodes, &path);
                let co_members = self.remove_co_members_from_key(key);
                displaced.extend(co_members);
                return (key, displaced);
            }
            path.push(p_id);
            parent_opt = gnodes.get(p_id.index()).parent();
        }

        panic!("catalytic_split: no basis element covers g_id");
    }

    fn reinsert_split_targets(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        displaced: &[GNodeId],
        g_id: GNodeId,
        left_id: GNodeId,
        right_id: GNodeId,
        left_depth: u32,
        right_depth: u32,
    ) {
        let mut to_place = Vec::new();
        for &sib_id in displaced {
            self.collect_subtree_basis_elements(gnodes, sib_id, &mut to_place);
        }
        if left_depth == right_depth {
            to_place.push((g_id, left_depth));
        } else {
            to_place.push((left_id, left_depth));
            to_place.push((right_id, right_depth));
        }
        self.place_sorted(gnodes, &mut to_place);
    }

    pub(super) fn on_catalytic_split_impl(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        g_id: GNodeId,
        left_id: GNodeId,
    ) {
        let _span = tracing::debug_span!(
            "plateau_after_catalytic_split",
            g_id = g_id.index(),
            left = left_id.index(),
        )
        .entered();

        self.plateaus_dirty = true;

        let (right_id, left_depth, right_depth) = self.catalytic_split_depths(gnodes, g_id, left_id);

        // ── Phase 1: Locate the covering basis element for g_id ─────────────────
        let (old_key, displaced) = self.locate_covering_basis_for_catalytic_split(gnodes, g_id);

        // ── Phase 2: Fixup the vacated plateau key ───────────────────────────
        self.fixup_plateau(gnodes, old_key);

        // ── Phase 3: Collect displaced elements and re-place ───────────────────
        self.reinsert_split_targets(
            gnodes,
            &displaced,
            g_id,
            left_id,
            right_id,
            left_depth,
            right_depth,
        );
    }
}
