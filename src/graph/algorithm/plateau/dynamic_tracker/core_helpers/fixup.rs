use super::super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::{GNode, GState};
use crate::spatial::plateau::BasisEdge;
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::{gnode_depth_from_interval, uniform_contour_depth_of};

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(in super::super) fn recompute_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, key: &BasisEdge<C>) {
        let elements = self.plateau_basis.basis_elements(key);
        if elements.is_empty() {
            return;
        }

        let first = *elements.iter().next().unwrap();
        let mut min_lo = gnodes.get(first.index()).lo();
        let mut max_hi = gnodes.get(first.index()).hi();
        let mut sum = V::zero();
        let mut depth = 0u32;

        for &gid in elements {
            let g = gnodes.get(gid.index());
            if g.lo().total_cmp(&min_lo) == std::cmp::Ordering::Less {
                min_lo = g.lo();
            }
            if g.hi().total_cmp(&max_hi) == std::cmp::Ordering::Greater {
                max_hi = g.hi();
            }
            sum = V::add(sum, g.sum());

            let d = match g.state() {
                GState::Terminal | GState::SemiInternal => {
                    gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits)
                }
                GState::Internal => uniform_contour_depth_of(gnodes, gid, self.n_bits)
                    .unwrap_or_else(|| gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits) + 1),
            };
            depth = depth.max(d);
        }

        if let Some(p) = self.plateaus.get_mut(key) {
            p.start = min_lo;
            p.end = max_hi;
            p.sum = sum;
            p.depth = depth;
        }
    }

    pub(in super::super) fn fixup_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, old_key: BasisEdge<C>) {
        use crate::spatial::plateau::{BasisEdge, Plateau, basis_edge_of};

        let element_ids: Vec<GNodeId> = self
            .plateau_basis
            .basis_elements(&old_key)
            .iter()
            .copied()
            .collect();

        if element_ids.is_empty() {
            self.plateaus.remove(&old_key);
            return;
        }

        if element_ids.len() > 1 {
            let mut intervals: Vec<(GNodeId, C, C)> = element_ids
                .iter()
                .map(|&gid| {
                    let g = gnodes.get(gid.index());
                    (gid, g.lo(), g.hi())
                })
                .collect();
            intervals.sort_by(|a, b| a.1.total_cmp(&b.1));

            let contiguous = intervals
                .windows(2)
                .all(|w| w[0].2.total_cmp(&w[1].1) == std::cmp::Ordering::Equal);

            if !contiguous {
                tracing::debug!(
                    ?old_key,
                    n_elements = element_ids.len(),
                    "fixup_plateau: non-contiguous remainder, evacuating"
                );
                let mut displaced: Vec<(GNodeId, u32)> = Vec::with_capacity(element_ids.len());
                for &gid in &element_ids {
                    self.plateau_basis.remove(gid);
                    let g = gnodes.get(gid.index());
                    let d = match g.state() {
                        GState::Terminal | GState::SemiInternal => {
                            gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits)
                        }
                        GState::Internal => uniform_contour_depth_of(gnodes, gid, self.n_bits)
                            .unwrap_or_else(|| {
                                gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits) + 1
                            }),
                    };
                    displaced.push((gid, d));
                }
                self.plateaus.remove(&old_key);
                self.place_sorted(gnodes, &mut displaced);
                return;
            }
        }

        let min_key: BasisEdge<C> = element_ids
            .iter()
            .map(|&gid| basis_edge_of(gnodes.get(gid.index())))
            .min()
            .unwrap();

        if min_key == old_key {
            self.recompute_plateau(gnodes, &old_key);
        } else {
            for &eid in &element_ids {
                self.plateau_basis.remove(eid);
            }
            self.plateaus.remove(&old_key);
            for &eid in &element_ids {
                self.plateau_basis.insert(min_key, eid);
            }
            self.plateaus.insert(
                min_key,
                Plateau {
                    basis_edge: min_key,
                    start: C::zero(),
                    end: C::zero(),
                    depth: 0,
                    sum: V::zero(),
                },
            );
            self.recompute_plateau(gnodes, &min_key);
        }
    }

    pub(in super::super) fn split_for_p_i4(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        parent_pk: BasisEdge<C>,
        child_id: GNodeId,
    ) {
        use crate::spatial::plateau::{Plateau, basis_edge_of};

        let boundary_id = self.find_boundary_node(gnodes, child_id, parent_pk);
        let Some(boundary_id) = boundary_id else {
            return;
        };

        let bg = gnodes.get(boundary_id.index());
        let boundary_key = basis_edge_of(bg);
        let boundary_depth = match bg.state() {
            GState::Terminal | GState::SemiInternal => {
                gnode_depth_from_interval(bg.lo(), bg.hi(), self.n_bits)
            }
            GState::Internal => gnode_depth_from_interval(bg.lo(), bg.hi(), self.n_bits) + 1,
        };
        let b_lo = bg.lo();
        let b_hi = bg.hi();
        let b_sum = bg.sum();

        if let Some(old_pk) = self.plateau_basis.remove(boundary_id) {
            self.fixup_plateau(gnodes, old_pk);
        }

        self.plateau_basis.insert(boundary_key, boundary_id);
        if let std::collections::btree_map::Entry::Vacant(e) = self.plateaus.entry(boundary_key) {
            e.insert(Plateau {
                basis_edge: boundary_key,
                start: b_lo,
                end: b_hi,
                depth: boundary_depth,
                sum: b_sum,
            });
        }
        self.recompute_plateau(gnodes, &boundary_key);
    }

    pub(in super::super) fn find_boundary_node(
        &self,
        gnodes: &Arena<GNode<C, V>>,
        gid: GNodeId,
        parent_pk: BasisEdge<C>,
    ) -> Option<GNodeId> {
        use crate::spatial::plateau::basis_edge_of;

        let g = gnodes.get(gid.index());
        let key = basis_edge_of(g);
        if key > parent_pk {
            return Some(gid);
        }

        if let Some(left) = g.left() {
            if let Some(found) = self.find_boundary_node(gnodes, left, parent_pk) {
                return Some(found);
            }
        }
        if let Some(right) = g.right() {
            if let Some(found) = self.find_boundary_node(gnodes, right, parent_pk) {
                return Some(found);
            }
        }
        None
    }

    /// Walks the ancestor chain upward from `parent_id` to find the basis
    /// element that covers it, removes that ancestor from the basis, and
    /// displaces the siblings of every path node into `displaced`.
    ///
    /// Returns the covering basis-edge key, or `None` if no ancestor in the
    /// basis covers `parent_id`.
    pub(in super::super) fn evict_ancestor_key(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        parent_id: GNodeId,
        parent_state_after: GState,
        displaced: &mut Vec<GNodeId>,
    ) -> Option<BasisEdge<C>> {
        use crate::spatial::plateau::basis_edge_of;

        let parent_key = basis_edge_of(gnodes.get(parent_id.index()));
        let mut path: Vec<GNodeId> = vec![parent_id];
        let mut cur = gnodes.get(parent_id.index()).parent();
        while let Some(anc) = cur {
            if let Some(anc_key) = self.covering_ancestor_key(anc, parent_key) {
                self.plateau_basis.remove(anc);
                self.displace_semi_internal_survivor(gnodes, parent_id, parent_state_after, displaced);
                self.displace_path_siblings(gnodes, &path, displaced);
                return Some(anc_key);
            }
            path.push(anc);
            cur = gnodes.get(anc.index()).parent();
        }

        None
    }

    fn covering_ancestor_key(&self, anc: GNodeId, parent_key: BasisEdge<C>) -> Option<BasisEdge<C>> {
        let anc_key = self.plateau_basis.plateau_key(anc)?;
        let next_key = self
            .plateaus
            .range((
                std::ops::Bound::Excluded(anc_key),
                std::ops::Bound::Unbounded,
            ))
            .next()
            .map(|(&k, _)| k);
        let tile_covers = parent_key >= anc_key && next_key.is_none_or(|nk| parent_key < nk);
        tile_covers.then_some(anc_key)
    }

    fn displace_semi_internal_survivor(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        parent_id: GNodeId,
        parent_state_after: GState,
        displaced: &mut Vec<GNodeId>,
    ) {
        if parent_state_after != GState::SemiInternal {
            return;
        }

        let g = gnodes.get(parent_id.index());
        if let Some(sib) = g.left().or_else(|| g.right()) {
            if let Some(ok) = self.plateau_basis.remove(sib) {
                self.fixup_plateau(gnodes, ok);
            }
            displaced.push(sib);
        }
    }

    fn displace_path_siblings(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        path: &[GNodeId],
        displaced: &mut Vec<GNodeId>,
    ) {
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
                if displaced.contains(&sib_id) {
                    continue;
                }
                if let Some(ok) = self.plateau_basis.remove(sib_id) {
                    self.fixup_plateau(gnodes, ok);
                }
                displaced.push(sib_id);
            }
        }
    }

    /// Evacuates right-adjacent and left-adjacent same-depth plateaus bordering
    /// the post-eviction parent, collecting their members into `displaced_extra`
    /// for subsequent re-placement.
    pub(in super::super) fn evacuate_adjacent_plateaus(
        &mut self,
        gnodes: &Arena<GNode<C, V>>,
        parent_lo: C,
        parent_hi: C,
        parent_depth: u32,
        displaced_extra: &mut Vec<(GNodeId, u32)>,
    ) {
        let parent_be = BasisEdge(parent_lo);

        // ── Phase 4: Evacuate right-adjacent same-depth plateaus ─────────────
        let right_keys: Vec<BasisEdge<C>> = self
            .plateaus
            .range(BasisEdge(parent_hi)..)
            .take_while(|(_, p)| p.start.total_cmp(&parent_hi) != std::cmp::Ordering::Greater)
            .filter(|(_, p)| p.depth == parent_depth)
            .map(|(&k, _)| k)
            .collect();
        for rk in right_keys {
            let members: Vec<GNodeId> = self
                .plateau_basis
                .basis_elements(&rk)
                .iter()
                .copied()
                .collect();
            if !members.is_empty() {
                tracing::trace!(
                    ?rk,
                    parent_depth,
                    members = ?members.iter().map(|g| g.index()).collect::<Vec<_>>(),
                    "evacuating right-adjacent same-depth plateau for evict placement",
                );
                for &m in &members {
                    self.plateau_basis.remove(m);
                }
                self.plateaus.remove(&rk);
                for &m in &members {
                    self.collect_subtree_basis_elements(gnodes, m, displaced_extra);
                }
            }
        }

        // ── Phase 5: Evacuate left-adjacent same-depth plateaus ──────────────
        let left_keys: Vec<BasisEdge<C>> = self
            .plateaus
            .range(..parent_be)
            .rev()
            .take_while(|(_, p)| p.end.total_cmp(&parent_lo) != std::cmp::Ordering::Less)
            .filter(|(_, p)| p.depth == parent_depth)
            .map(|(&k, _)| k)
            .collect();
        for lk in left_keys {
            let members: Vec<GNodeId> = self
                .plateau_basis
                .basis_elements(&lk)
                .iter()
                .copied()
                .collect();
            if !members.is_empty() {
                tracing::trace!(
                    ?lk,
                    parent_depth,
                    members = ?members.iter().map(|g| g.index()).collect::<Vec<_>>(),
                    "evacuating left-adjacent same-depth plateau for evict placement",
                );
                for &m in &members {
                    self.plateau_basis.remove(m);
                }
                self.plateaus.remove(&lk);
                for &m in &members {
                    self.collect_subtree_basis_elements(gnodes, m, displaced_extra);
                }
            }
        }
    }
}
