use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::graph::GvGraph;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::{Accumulator, Coordinate, Inspectable, PlateauTracking};
use crate::tree::gtree::GTree;

impl<C: Coordinate, V: Accumulator + Inspectable, const N: u32, T: PlateauTracking<C, V>>
    GvGraph<C, V, N, T>
{
    /// Returns the current plateau map.
    ///
    /// With `dynamic-contour-tracking` this returns a borrowed reference to
    /// the incrementally-maintained mirror; without the feature it rebuilds the
    /// map from the G-tree on every call.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn plateaus(&self) -> Cow<'_, BTreeMap<BasisEdge<C>, Plateau<C, V>>> {
        self.tracker.plateaus()
    }

    /// Rebuilds the plateau map by walking the G-tree statically.
    ///
    /// Used for consistency checks and as the implementation of `plateaus()`
    /// when `dynamic-contour-tracking` is disabled.
    #[doc(hidden)]
    #[must_use]
    pub fn build_plateaus(&self) -> BTreeMap<BasisEdge<C>, Plateau<C, V>> {
        use crate::spatial::plateau::basis_edge_of;
        use crate::tree::gtree::gnode::GState;

        let mut basis: Vec<(BasisEdge<C>, u32, C, C, V)> = Vec::new();
        let mut stack = vec![self.core.gtree.nodes.root];
        while let Some(gid) = stack.pop() {
            let g = self.core.gtree.nodes.get(gid.index());
            match g.state() {
                GState::Terminal => {
                    let depth = GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi());
                    basis.push((BasisEdge(g.lo()), depth, g.lo(), g.hi(), g.sum()));
                }
                GState::SemiInternal => {
                    let depth = GTree::<C, V, N>::depth_of_interval(g.lo(), g.hi());
                    basis.push((basis_edge_of(g), depth, g.lo(), g.hi(), g.sum()));

                    if let Some(left) = g.left() {
                        stack.push(left);
                    }
                    if let Some(right) = g.right() {
                        stack.push(right);
                    }
                }
                GState::Internal => {
                    if let Some(ud) = self.core.gtree.uniform_contour_depth(gid) {
                        basis.push((basis_edge_of(g), ud, g.lo(), g.hi(), g.sum()));
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

        basis.sort_by_key(|a| a.0);

        let mut result: BTreeMap<BasisEdge<C>, Plateau<C, V>> = BTreeMap::new();
        for (be, depth, lo, hi, sum) in basis {
            let merged = if let Some((_, prev)) = result.iter_mut().next_back() {
                if prev.depth == depth {
                    if hi.total_cmp(&prev.end) == std::cmp::Ordering::Greater {
                        prev.end = hi;
                    }
                    if lo.total_cmp(&prev.start) == std::cmp::Ordering::Less {
                        prev.start = lo;
                    }
                    prev.sum = V::add(prev.sum, sum);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if !merged {
                result.insert(
                    be,
                    Plateau {
                        basis_edge: be,
                        start: lo,
                        end: hi,
                        depth,
                        sum,
                    },
                );
            }
        }

        result
    }

    #[must_use]
    pub fn select_plateaus(&self, lo: C, hi: C) -> Option<(BasisEdge<C>, BasisEdge<C>)> {
        use std::ops::Bound;

        if lo >= hi {
            return None;
        }

        let plateaus = self.plateaus();
        if plateaus.is_empty() {
            return None;
        }

        let start_entry = plateaus
            .range(..=BasisEdge(lo))
            .next_back()
            .map(|(k, _)| *k)?;

        let last_plateau_key = plateaus
            .range(..BasisEdge(hi))
            .next_back()
            .map(|(k, _)| *k)?;

        let end = plateaus
            .range((Bound::Excluded(last_plateau_key), Bound::Unbounded))
            .next()
            .map_or_else(|| BasisEdge(C::domain_max(N)), |(k, _)| *k);

        if start_entry >= end {
            return None;
        }

        Some((start_entry, end))
    }
}
