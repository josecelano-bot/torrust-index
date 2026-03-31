use std::collections::BTreeMap;

use super::DynamicPlateauTracker;
use crate::handle::GNodeId;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GNodeTree;

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    #[allow(clippy::too_many_lines, clippy::float_cmp)]
    pub(super) fn normalize_impl(&mut self, gnodes: &GNodeTree<C, V>) {
        if !self.plateaus_dirty {
            return;
        }
        self.plateaus_dirty = false;

        let _span = tracing::debug_span!("normalize_plateaus").entered();

        #[cfg(debug_assertions)]
        let old_total: V = self
            .plateaus
            .values()
            .map(|p| p.sum)
            .fold(V::zero(), V::add);

        // ── Phase 1: Collect and expand basis elements (DFS per basis node) ───────
        let mut elems = self.collect_normalize_elements(gnodes);
        // ── Phase 2: Sort by basis-edge key ─────────────────────────────────────
        elems.sort_by_key(|e| e.1);

        #[cfg(debug_assertions)]
        {
            let mut ids: Vec<usize> = elems.iter().map(|e| e.0.index()).collect();
            ids.sort_unstable();
            for w in ids.windows(2) {
                debug_assert_ne!(
                    w[0], w[1],
                    "normalize_plateaus step 1: duplicate GNodeId({}) in DFS collection",
                    w[0],
                );
            }
        }

        // ── Phase 3: Sweep-merge adjacent same-depth tiles ───────────────────────
        let mut new_plateaus: BTreeMap<BasisEdge<C>, Plateau<C, V>> = BTreeMap::new();
        let mut assignments: Vec<(BasisEdge<C>, GNodeId)> = Vec::with_capacity(elems.len());

        for (gid, be, depth, lo, hi, sum) in &elems {
            let merge_key = new_plateaus
                .range(..*be)
                .next_back()
                .filter(|(_, p)| p.depth == *depth)
                .map(|(&k, _)| k);

            let key = if let Some(mk) = merge_key {
                let p = new_plateaus.get_mut(&mk).unwrap();
                if hi.total_cmp(&p.end) == std::cmp::Ordering::Greater {
                    p.end = *hi;
                }
                if lo.total_cmp(&p.start) == std::cmp::Ordering::Less {
                    p.start = *lo;
                }
                p.sum = V::add(p.sum, *sum);
                mk
            } else {
                new_plateaus.insert(
                    *be,
                    Plateau {
                        basis_edge: *be,
                        start: *lo,
                        end: *hi,
                        depth: *depth,
                        sum: *sum,
                    },
                );
                *be
            };
            assignments.push((key, *gid));
        }

        #[cfg(debug_assertions)]
        {
            let mut key_sums: BTreeMap<BasisEdge<C>, V> = BTreeMap::new();
            for (i, (key, _gid)) in assignments.iter().enumerate() {
                let entry = key_sums.entry(*key).or_insert(V::zero());
                *entry = V::add(*entry, elems[i].5);
            }
            for (key, &elem_sum) in &key_sums {
                let sweep_sum = new_plateaus[key].sum;
                debug_assert_eq!(
                    sweep_sum, elem_sum,
                    "normalize_plateaus step 2: sweep sum mismatch for plateau {key:?}",
                );
            }
        }

        // ── Phase 4: Install new plateau map, rebuild basis, and consolidate ──────
        self.plateaus = new_plateaus;
        self.plateau_basis.rebuild(assignments);

        self.consolidate_all_basis(gnodes);

        #[cfg(debug_assertions)]
        {
            let new_total: V = self
                .plateaus
                .values()
                .map(|p| p.sum)
                .fold(V::zero(), V::add);
            debug_assert_eq!(
                old_total, new_total,
                "normalize_plateaus: total plateau energy changed after consolidation",
            );
        }
    }
}
