use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::GNode;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::gnode_depth_from_interval;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
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

        let right_id = gnodes
            .get(g_id.index())
            .right()
            .expect("catalytic_split: g_id must have a right child");

        let left_depth = gnode_depth_from_interval(
            gnodes.get(left_id.index()).lo(),
            gnodes.get(left_id.index()).hi(),
            self.n_bits,
        );
        let right_depth = gnode_depth_from_interval(
            gnodes.get(right_id.index()).lo(),
            gnodes.get(right_id.index()).hi(),
            self.n_bits,
        );

        // ── Phase 1: Locate the covering basis element for g_id ─────────────────
        let (old_key, displaced) = if let Some(key) = self.plateau_basis.remove(g_id) {
            let co_members: Vec<GNodeId> = self
                .plateau_basis
                .basis_elements(&key)
                .iter()
                .copied()
                .collect();
            for &m in &co_members {
                self.plateau_basis.remove(m);
            }
            (key, co_members)
        } else {
            let mut path: Vec<GNodeId> = vec![g_id];
            let mut parent_opt = gnodes.get(g_id.index()).parent();
            let mut result = None;

            while let Some(p_id) = parent_opt {
                if let Some(key) = self.plateau_basis.remove(p_id) {
                    let mut displaced: Vec<GNodeId> = Vec::new();
                    for &path_node in &path {
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

                    let co_members: Vec<GNodeId> = self
                        .plateau_basis
                        .basis_elements(&key)
                        .iter()
                        .copied()
                        .collect();
                    for &m in &co_members {
                        self.plateau_basis.remove(m);
                    }
                    displaced.extend(co_members);
                    result = Some((key, displaced));
                    break;
                }
                path.push(p_id);
                parent_opt = gnodes.get(p_id.index()).parent();
            }

            result.expect("catalytic_split: no basis element covers g_id")
        };

        // ── Phase 2: Fixup the vacated plateau key ───────────────────────────
        self.fixup_plateau(gnodes, old_key);

        // ── Phase 3: Collect displaced elements and re-place ───────────────────
        let mut to_place = Vec::new();
        for &sib_id in &displaced {
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
}
