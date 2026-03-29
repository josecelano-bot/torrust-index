// Shared helper methods for DynamicPlateauTracker.

use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::handle::GNodeId;
use crate::nodes::gnode::{GNode, GState};
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::{gnode_depth_from_interval, uniform_contour_depth_of};

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {

pub(super) fn recompute_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, key: &BasisEdge<C>) {
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

/// Inserts `gnode` at the given `depth` into the plateau basis, merging
/// with adjacent same-depth plateaus where possible.
pub(super) fn place_basis_element(&mut self, gnodes: &Arena<GNode<C, V>>, gnode: GNodeId, depth: u32) {
    use crate::spatial::plateau::basis_edge_of;

    let g = gnodes.get(gnode.index());
    let key = basis_edge_of(g);
    let lo = g.lo();
    let hi = g.hi();

    let left_key = self.find_adjacent_left_key(key, lo, depth);
    let right_key = self.find_adjacent_right_key(key, hi, depth);

    match (left_key, right_key) {
        (Some(lk), Some(rk)) => self.place_merge_both(gnodes, lk, rk, gnode),
        (Some(lk), None)     => self.place_extend_left(gnodes, lk, gnode),
        (None, Some(rk))     => self.place_rekey_right(gnodes, key, rk, gnode, depth),
        (None, None)         => self.place_new_plateau(gnodes, key, gnode, depth),
    }

    self.consolidate_basis_up(gnodes, gnode);

    if gnodes.get(gnode.index()).state() == GState::SemiInternal {
        let final_key = self.plateau_basis.plateau_key(gnode).expect("just placed");
        self.pending_p_i4.push((gnode, final_key));
    }
}

pub(super) fn collect_subtree_basis_elements(
    &self,
    gnodes: &Arena<GNode<C, V>>,
    gid: GNodeId,
    out: &mut Vec<(GNodeId, u32)>,
) {
    let g = gnodes.get(gid.index());
    match g.state() {
        GState::Terminal | GState::SemiInternal => {
            let depth = gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits);
            out.push((gid, depth));
        }
        GState::Internal => {
            if let Some(ud) = uniform_contour_depth_of(gnodes, gid, self.n_bits) {
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

pub(super) fn place_sorted(&mut self, gnodes: &Arena<GNode<C, V>>, elements: &mut [(GNodeId, u32)]) {
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

pub(super) fn fixup_plateau(&mut self, gnodes: &Arena<GNode<C, V>>, old_key: BasisEdge<C>) {
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

pub(super) fn split_for_p_i4(
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

pub(super) fn find_boundary_node(
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

/// Walks upward from `gid`, merging sibling basis-element pairs into their
/// parent whenever the subtree is uniform-depth.
#[allow(clippy::too_many_lines)]
pub(super) fn consolidate_basis_up(&mut self, gnodes: &Arena<GNode<C, V>>, mut gid: GNodeId) {
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

        let Some(uniform_depth) = uniform_contour_depth_of(gnodes, parent_id, self.n_bits)
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
pub(super) fn consolidate_all_basis(&mut self, gnodes: &Arena<GNode<C, V>>) {
    let basis_snapshot: Vec<GNodeId> = self.plateau_basis.back_map().keys().copied().collect();
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

// ── place_basis_element helpers ───────────────────────────────────────────

/// Returns the key of the adjacent left plateau at the same `depth` that
/// ends at or after `lo`, with no intervening plateau between it and `key`.
pub(super) fn find_adjacent_left_key(
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
pub(super) fn find_adjacent_right_key(
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
pub(super) fn place_merge_both(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
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
    tracing::trace!(gnode = gnode.index(), ?lk, ?rk, "place_basis_element: merge-both");
}

/// Extends the existing left plateau `lk` to include `gnode`.
pub(super) fn place_extend_left(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
    lk: BasisEdge<C>,
    gnode: GNodeId,
) {
    self.plateau_basis.insert(lk, gnode);
    self.recompute_plateau(gnodes, &lk);
    tracing::trace!(gnode = gnode.index(), ?lk, "place_basis_element: insert-left");
}

/// Re-keys the right plateau `rk` to `key` and inserts `gnode` into it.
pub(super) fn place_rekey_right(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
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
    tracing::trace!(gnode = gnode.index(), ?rk, "place_basis_element: rekey-right");
}

/// Creates a new plateau at `key` for `gnode`, or recomputes it if
/// the key already exists.
pub(super) fn place_new_plateau(
    &mut self,
    gnodes: &Arena<GNode<C, V>>,
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

// ── on_evict helpers ──────────────────────────────────────────────────────

/// Walks the ancestor chain upward from `parent_id` to find the basis
/// element that covers it, removes that ancestor from the basis, and
/// displaces the siblings of every path node into `displaced`.
///
/// Returns the covering basis-edge key, or `None` if no ancestor in the
/// basis covers `parent_id`.
pub(super) fn evict_ancestor_key(
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
    let mut found = None;

    while let Some(anc) = cur {
        if let Some(&anc_key) = self.plateau_basis.plateau_key(anc).as_ref() {
            let next_key = self
                .plateaus
                .range((
                    std::ops::Bound::Excluded(anc_key),
                    std::ops::Bound::Unbounded,
                ))
                .next()
                .map(|(&k, _)| k);
            let tile_covers =
                parent_key >= anc_key && next_key.is_none_or(|nk| parent_key < nk);

            if tile_covers {
                self.plateau_basis.remove(anc);

                if parent_state_after == GState::SemiInternal {
                    let g = gnodes.get(parent_id.index());
                    if let Some(sib) = g.left().or_else(|| g.right()) {
                        if let Some(ok) = self.plateau_basis.remove(sib) {
                            self.fixup_plateau(gnodes, ok);
                        }
                        displaced.push(sib);
                    }
                }

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
                        if displaced.contains(&sib_id) {
                            continue;
                        }
                        if let Some(ok) = self.plateau_basis.remove(sib_id) {
                            self.fixup_plateau(gnodes, ok);
                        }
                        displaced.push(sib_id);
                    }
                }

                found = Some(anc_key);
                break;
            }
        }
        path.push(anc);
        cur = gnodes.get(anc.index()).parent();
    }

    found
}

/// Evacuates right-adjacent and left-adjacent same-depth plateaus bordering
/// the post-eviction parent, collecting their members into `displaced_extra`
/// for subsequent re-placement.
pub(super) fn evacuate_adjacent_plateaus(
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

// ── normalize helpers ─────────────────────────────────────────────────────

/// Collects and expands all current basis elements into a flat list of
/// `(id, basis_edge, depth, lo, hi, sum)` tuples sorted for sweep-merging.
///
/// Internal nodes with a uniform contour depth are treated as single
/// entries; those without are recursively expanded via a DFS stack.
pub(super) fn collect_normalize_elements(
    &self,
    gnodes: &Arena<GNode<C, V>>,
) -> Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)> {
    use crate::spatial::plateau::basis_edge_of;

    let mut elems: Vec<(GNodeId, BasisEdge<C>, u32, C, C, V)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let basis_ids: Vec<GNodeId> = self.plateau_basis.back_map().keys().copied().collect();

    for gid in basis_ids {
        let mut stack = vec![gid];
        while let Some(nid) = stack.pop() {
            if !seen.insert(nid) {
                continue;
            }
            let g = gnodes.get(nid.index());
            match g.state() {
                GState::Terminal => {
                    let depth = gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits);
                    elems.push((nid, basis_edge_of(g), depth, g.lo(), g.hi(), g.sum()));
                }
                GState::SemiInternal => {
                    let depth = gnode_depth_from_interval(g.lo(), g.hi(), self.n_bits);
                    elems.push((nid, basis_edge_of(g), depth, g.lo(), g.hi(), g.sum()));
                    if let Some(left) = g.left() {
                        stack.push(left);
                    }
                    if let Some(right) = g.right() {
                        stack.push(right);
                    }
                }
                GState::Internal => {
                    if let Some(ud) = uniform_contour_depth_of(gnodes, nid, self.n_bits) {
                        elems.push((nid, basis_edge_of(g), ud, g.lo(), g.hi(), g.sum()));
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
    }

    elems
}

}
