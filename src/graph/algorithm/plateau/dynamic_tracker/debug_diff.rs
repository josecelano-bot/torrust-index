use std::collections::BTreeMap;

use super::DynamicPlateauTracker;
use crate::arena::Arena;
use crate::nodes::gnode::GNode;
use crate::spatial::plateau::{BasisEdge, Plateau};
use crate::traits::{Accumulator, Coordinate};

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(super) fn debug_assert_mirror_consistency_impl(
        &self,
        gnodes: &Arena<GNode<C, V>>,
        fresh: &BTreeMap<BasisEdge<C>, Plateau<C, V>>,
        label: &str,
    ) {
        if !cfg!(debug_assertions) && !tracing::enabled!(tracing::Level::DEBUG) {
            return;
        }
        let dynamic = &self.plateaus;
        if dynamic == fresh {
            return;
        }
        let dyn_keys: std::collections::BTreeSet<_> = dynamic.keys().collect();
        let stat_keys: std::collections::BTreeSet<_> = fresh.keys().collect();
        let only_dynamic: Vec<_> = dyn_keys.difference(&stat_keys).collect();
        let only_static: Vec<_> = stat_keys.difference(&dyn_keys).collect();
        let both: Vec<_> = dyn_keys.intersection(&stat_keys).collect();
        let differing: Vec<_> = both
            .iter()
            .filter(|&&k| dynamic.get(k) != fresh.get(k))
            .collect();

        tracing::error!(
            label,
            only_dynamic = ?only_dynamic,
            only_static = ?only_static,
            differing = ?differing,
            "PLATEAU DIVERGENCE DETECTED",
        );

        for &&key in &only_dynamic {
            let p = &dynamic[key];
            let basis = self.plateau_basis.basis_elements(key);
            tracing::error!(
                key = ?key,
                depth = p.depth,
                start = ?p.start,
                end = ?p.end,
                sum = ?p.sum,
                basis = ?basis.iter().map(|g| g.index()).collect::<Vec<_>>(),
                "DYNAMIC-ONLY plateau",
            );
            for &gid in basis {
                if gnodes.is_occupied(gid.index()) {
                    let g = gnodes.get(gid.index());
                    tracing::error!(
                        gid = gid.index(),
                        state = ?g.state(),
                        lo = ?g.lo(),
                        hi = ?g.hi(),
                        sum = ?g.sum(),
                        "  basis element",
                    );
                }
            }
        }

        for &&key in &only_static {
            let p = &fresh[key];
            tracing::error!(
                key = ?key,
                depth = p.depth,
                start = ?p.start,
                end = ?p.end,
                sum = ?p.sum,
                "STATIC-ONLY plateau",
            );
        }

        for &&&key in &differing {
            let dyn_p = &dynamic[key];
            let stat_p = &fresh[key];
            let basis = self.plateau_basis.basis_elements(key);
            tracing::error!(
                key = ?key,
                dyn_depth = dyn_p.depth,
                dyn_start = ?dyn_p.start,
                dyn_end = ?dyn_p.end,
                dyn_sum = ?dyn_p.sum,
                stat_depth = stat_p.depth,
                stat_start = ?stat_p.start,
                stat_end = ?stat_p.end,
                stat_sum = ?stat_p.sum,
                basis = ?basis.iter().map(|g| g.index()).collect::<Vec<_>>(),
                "DIFFERS",
            );
            for &gid in basis {
                if gnodes.is_occupied(gid.index()) {
                    let g = gnodes.get(gid.index());
                    tracing::error!(
                        gid = gid.index(),
                        state = ?g.state(),
                        lo = ?g.lo(),
                        hi = ?g.hi(),
                        sum = ?g.sum(),
                        "  basis element",
                    );
                }
            }
        }

        panic!(
            "{label}: dynamic-contour-tracking mirror diverged from static rebuild\n\
             left (dynamic): {dynamic:#?}\n\
             right (static): {fresh:#?}"
        );
    }
}
