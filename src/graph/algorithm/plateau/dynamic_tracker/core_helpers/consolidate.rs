use super::super::DynamicPlateauTracker;
use crate::handle::GNodeId;
use crate::spatial::plateau::BasisEdge;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GNodeTree;
use crate::tree::gtree::gnode::{GNode, GState};
use crate::tree::gtree::gnode_depth_from_range;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    fn basis_ids_snapshot(&self) -> Vec<GNodeId> {
        self.plateau_basis.back_map().keys().copied().collect()
    }

    fn push_normalize_element(
        &self,
        gnodes: &GNodeTree<C, V>,
        elems: &mut Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)>,
        nid: GNodeId,
        depth: u32,
    ) {
        use crate::spatial::plateau::basis_edge_of;

        let g = gnodes.get(nid.index());
        elems.push((nid, basis_edge_of(g), depth, g.lo(), g.hi(), g.sum()));
    }

    fn process_normalize_node(
        &self,
        gnodes: &GNodeTree<C, V>,
        nid: GNodeId,
        elems: &mut Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)>,
        stack: &mut Vec<GNodeId>,
    ) {
        let g: &GNode<C, V> = gnodes.get(nid.index());
        match g.state() {
            GState::Terminal => {
                let depth = gnode_depth_from_range(g.range(), self.n_bits);
                self.push_normalize_element(gnodes, elems, nid, depth);
            }
            GState::SemiInternal => {
                let depth = gnode_depth_from_range(g.range(), self.n_bits);
                self.push_normalize_element(gnodes, elems, nid, depth);
                if let Some(left) = g.left() {
                    stack.push(left);
                }
                if let Some(right) = g.right() {
                    stack.push(right);
                }
            }
            GState::Internal => {
                if let Some(ud) = gnodes.uniform_contour_depth_of(nid, self.n_bits) {
                    self.push_normalize_element(gnodes, elems, nid, ud);
                } else {
                    if let Some(left) = g.left() {
                        stack.push(left);
                    }
                    if let Some(right) = g.right() {
                        stack.push(right);
                    }
                }
            }
        }
    }

    fn collect_from_basis_root(
        &self,
        gnodes: &GNodeTree<C, V>,
        gid: GNodeId,
        elems: &mut Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)>,
        seen: &mut std::collections::HashSet<GNodeId>,
    ) {
        let mut stack = vec![gid];
        while let Some(nid) = stack.pop() {
            if !seen.insert(nid) {
                continue;
            }
            self.process_normalize_node(gnodes, nid, elems, &mut stack);
        }
    }

    pub(in super::super) fn collect_subtree_basis_elements(
        &self,
        gnodes: &GNodeTree<C, V>,
        gid: GNodeId,
        out: &mut Vec<(GNodeId, u32)>,
    ) {
        let g = gnodes.get(gid.index());
        match g.state() {
            GState::Terminal | GState::SemiInternal => {
                let depth = gnode_depth_from_range(g.range(), self.n_bits);
                out.push((gid, depth));
            }
            GState::Internal => {
                if let Some(ud) = gnodes.uniform_contour_depth_of(gid, self.n_bits) {
                    out.push((gid, ud));
                } else {
                    if let Some(l) = g.left() {
                        self.collect_subtree_basis_elements(gnodes, l, out);
                    }
                    if let Some(r) = g.right() {
                        self.collect_subtree_basis_elements(gnodes, r, out);
                    }
                }
            }
        }
    }

    pub(in super::super) fn place_sorted(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        elements: &mut [(GNodeId, u32)],
    ) {
        use crate::spatial::plateau::basis_edge_of;
        elements.sort_by(|a, b| {
            let a_key = basis_edge_of(gnodes.get(a.0.index()));
            let b_key = basis_edge_of(gnodes.get(b.0.index()));
            a_key.cmp(&b_key)
        });
        for &(gid, depth) in elements.iter() {
            self.place_basis_element(gnodes, gid, depth);
        }
    }

    /// Walks upward from `gid`, merging sibling basis-element pairs into their
    /// parent whenever the subtree is uniform-depth.
    #[allow(clippy::too_many_lines)]
    pub(in super::super) fn consolidate_basis_up(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        mut gid: GNodeId,
    ) {
        loop {
            let Some(parent_id) = gnodes.get(gid.index()).parent() else {
                tracing::trace!(from = gid.index(), "consolidate_basis_up: stop — no parent");
                break;
            };
            if gnodes.get(parent_id.index()).state() != GState::Internal {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    parent_state = ?gnodes.get(parent_id.index()).state(),
                    "consolidate_basis_up: stop — parent not Internal",
                );
                break;
            }
            let (left, right) = {
                let pg = gnodes.get(parent_id.index());
                match (pg.left(), pg.right()) {
                    (Some(l), Some(r)) => (l, r),
                    _ => break,
                }
            };

            if gnodes.get(left.index()).state() == GState::SemiInternal
                || gnodes.get(right.index()).state() == GState::SemiInternal
            {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    "consolidate_basis_up: stop — semi-internal child"
                );
                break;
            }

            let Some(left_key) = self.plateau_basis.plateau_key(left) else {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    left = left.index(),
                    "consolidate_basis_up: stop — left not in basis"
                );
                break;
            };
            let Some(right_key) = self.plateau_basis.plateau_key(right) else {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    right = right.index(),
                    "consolidate_basis_up: stop — right not in basis"
                );
                break;
            };
            if left_key != right_key {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    ?left_key,
                    ?right_key,
                    "consolidate_basis_up: stop — children in different plateaus"
                );
                break;
            }

            let Some(uniform_depth) = gnodes.uniform_contour_depth_of(parent_id, self.n_bits)
            else {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    "consolidate_basis_up: stop — parent subtree not uniform"
                );
                break;
            };

            let Some(p) = self.plateaus.get(&left_key) else {
                break;
            };
            if p.depth != uniform_depth {
                tracing::trace!(
                    from = gid.index(),
                    parent = parent_id.index(),
                    plateau_depth = p.depth,
                    uniform_depth,
                    "consolidate_basis_up: stop — depth mismatch"
                );
                break;
            }

            tracing::debug!(
                left = left.index(),
                right = right.index(),
                parent = parent_id.index(),
                ?left_key,
                uniform_depth,
                "consolidate_basis_up: MERGING siblings into parent",
            );
            let key = left_key;
            self.plateau_basis.remove(left);
            self.plateau_basis.remove(right);
            self.plateau_basis.insert(key, parent_id);
            self.recompute_plateau(gnodes, &key);

            gid = parent_id;
        }
    }

    /// Iterates every basis element and attempts to merge sibling pairs into
    /// their parent via [`consolidate_basis_up`].
    pub(in super::super) fn consolidate_all_basis(&mut self, gnodes: &GNodeTree<C, V>) {
        let basis_snapshot = self.basis_ids_snapshot();
        let count = basis_snapshot.len();
        let mut merged = 0u32;
        for gid in basis_snapshot {
            if self.plateau_basis.plateau_key(gid).is_some() {
                let before = self.plateau_basis.basis_count();
                self.consolidate_basis_up(gnodes, gid);
                let after = self.plateau_basis.basis_count();
                if after < before {
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        merged += (before - after) as u32;
                    }
                }
            }
        }
        if merged > 0 {
            tracing::debug!(
                basis_before = count,
                basis_after = self.plateau_basis.basis_count(),
                merged,
                "consolidate_all_basis: completed",
            );
        }
    }

    /// Collects and expands all current basis elements into a flat list of
    /// `(id, basis_edge, depth, lo, hi, sum)` tuples sorted for sweep-merging.
    ///
    /// Internal nodes with a uniform contour depth are treated as single
    /// entries; those without are recursively expanded via a DFS stack.
    pub(in super::super) fn collect_normalize_elements(
        &self,
        gnodes: &GNodeTree<C, V>,
    ) -> Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)> {
        let mut elems: Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let basis_ids = self.basis_ids_snapshot();

        for gid in basis_ids {
            self.collect_from_basis_root(gnodes, gid, &mut elems, &mut seen);
        }

        elems
    }
}
