use super::super::DynamicPlateauTracker;
use crate::handle::GNodeId;
use crate::nodes::gnode::GState;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GNodeTree;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    /// Inserts `gnode` at the given `depth` into the plateau basis, merging
    /// with adjacent same-depth plateaus where possible.
    pub(in super::super) fn place_basis_element(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        gnode: GNodeId,
        depth: u32,
    ) {
        use crate::spatial::plateau::basis_edge_of;

        let g = gnodes.get(gnode.index());
        let key = basis_edge_of(g);
        let lo = g.lo();
        let hi = g.hi();

        let left_key = self.find_adjacent_left_key(key, lo, depth);
        let right_key = self.find_adjacent_right_key(key, hi, depth);

        match (left_key, right_key) {
            (Some(lk), Some(rk)) => self.place_merge_both(gnodes, lk, rk, gnode),
            (Some(lk), None) => self.place_extend_left(gnodes, lk, gnode),
            (None, Some(rk)) => self.place_rekey_right(gnodes, key, rk, gnode, depth),
            (None, None) => self.place_new_plateau(gnodes, key, gnode, depth),
        }

        self.consolidate_basis_up(gnodes, gnode);

        if gnodes.get(gnode.index()).state() == GState::SemiInternal {
            let final_key = self.plateau_basis.plateau_key(gnode).expect("just placed");
            self.pending_p_i4.push((gnode, final_key));
        }
    }

    /// Returns the key of the adjacent left plateau at the same `depth` that
    /// ends at or after `lo`, with no intervening plateau between it and `key`.
    pub(in super::super) fn find_adjacent_left_key(
        &self,
        key: BasisEdge<C>,
        lo: C,
        depth: u32,
    ) -> Option<BasisEdge<C>> {
        self.plateaus
            .range(..key)
            .next_back()
            .filter(|(_, p)| p.depth == depth && p.end.total_cmp(&lo) != std::cmp::Ordering::Less)
            .map(|(&k, _)| k)
            .filter(|lk| {
                self.plateaus
                    .range((
                        std::ops::Bound::Excluded(*lk),
                        std::ops::Bound::Excluded(key),
                    ))
                    .next()
                    .is_none()
            })
    }

    /// Returns the key of the adjacent right plateau at the same `depth` that
    /// starts at or before `hi`, with no intervening plateau between `key` and it.
    pub(in super::super) fn find_adjacent_right_key(
        &self,
        key: BasisEdge<C>,
        hi: C,
        depth: u32,
    ) -> Option<BasisEdge<C>> {
        self.plateaus
            .range(BasisEdge(hi)..)
            .next()
            .filter(|(_, p)| {
                p.depth == depth && p.start.total_cmp(&hi) != std::cmp::Ordering::Greater
            })
            .map(|(&k, _)| k)
            .filter(|rk| {
                self.plateaus
                    .range((
                        std::ops::Bound::Excluded(key),
                        std::ops::Bound::Excluded(*rk),
                    ))
                    .next()
                    .is_none()
            })
    }

    /// Merges `gnode` into the left plateau `lk`, absorbing the right plateau
    /// `rk` into `lk` at the same time.
    pub(in super::super) fn place_merge_both(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        lk: BasisEdge<C>,
        rk: BasisEdge<C>,
        gnode: GNodeId,
    ) {
        self.plateau_basis.insert(lk, gnode);
        let rights: Vec<_> = self
            .plateau_basis
            .basis_elements(&rk)
            .iter()
            .copied()
            .collect();
        for rid in rights {
            self.plateau_basis.remove(rid);
            self.plateau_basis.insert(lk, rid);
        }
        self.plateaus.remove(&rk);
        self.recompute_plateau(gnodes, &lk);
        tracing::trace!(
            gnode = gnode.index(),
            ?lk,
            ?rk,
            "place_basis_element: merge-both"
        );
    }

    /// Extends the existing left plateau `lk` to include `gnode`.
    pub(in super::super) fn place_extend_left(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        lk: BasisEdge<C>,
        gnode: GNodeId,
    ) {
        self.plateau_basis.insert(lk, gnode);
        self.recompute_plateau(gnodes, &lk);
        tracing::trace!(
            gnode = gnode.index(),
            ?lk,
            "place_basis_element: insert-left"
        );
    }

    /// Re-keys the right plateau `rk` to `key` and inserts `gnode` into it.
    pub(in super::super) fn place_rekey_right(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        key: BasisEdge<C>,
        rk: BasisEdge<C>,
        gnode: GNodeId,
        depth: u32,
    ) {
        let g = gnodes.get(gnode.index());
        let lo = g.lo();
        let sum = g.sum();
        let rights: Vec<_> = self
            .plateau_basis
            .basis_elements(&rk)
            .iter()
            .copied()
            .collect();
        for rid in rights {
            self.plateau_basis.remove(rid);
            self.plateau_basis.insert(key, rid);
        }
        self.plateau_basis.insert(key, gnode);
        self.plateaus.remove(&rk);
        self.plateaus.insert(
            key,
            Plateau {
                basis_edge: key,
                start: lo,
                end: lo,
                depth,
                sum,
            },
        );
        self.recompute_plateau(gnodes, &key);
        tracing::trace!(
            gnode = gnode.index(),
            ?rk,
            "place_basis_element: rekey-right"
        );
    }

    /// Creates a new plateau at `key` for `gnode`, or recomputes it if
    /// the key already exists.
    pub(in super::super) fn place_new_plateau(
        &mut self,
        gnodes: &GNodeTree<C, V>,
        key: BasisEdge<C>,
        gnode: GNodeId,
        depth: u32,
    ) {
        let g = gnodes.get(gnode.index());
        let lo = g.lo();
        let hi = g.hi();
        let sum = g.sum();
        self.plateau_basis.insert(key, gnode);
        if let std::collections::btree_map::Entry::Vacant(e) = self.plateaus.entry(key) {
            e.insert(Plateau {
                basis_edge: key,
                start: lo,
                end: hi,
                depth,
                sum,
            });
        } else {
            self.recompute_plateau(gnodes, &key);
        }
        tracing::trace!(gnode = gnode.index(), "place_basis_element: new-plateau");
    }
}
