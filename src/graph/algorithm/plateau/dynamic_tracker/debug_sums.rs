use super::DynamicPlateauTracker;
use crate::tree::gtree::GNodeTree;
use crate::traits::{Accumulator, Coordinate, Inspectable};

// ── Inspectable-bounded debug helpers ────────────────────────────────────────

impl<C: Coordinate, V: Accumulator + Inspectable> DynamicPlateauTracker<C, V> {
    #[allow(clippy::float_cmp)]
    pub(crate) fn debug_check_sums(&self, gnodes: &GNodeTree<C, V>, label: &str) {
        if !cfg!(debug_assertions) && !tracing::enabled!(tracing::Level::DEBUG) {
            return;
        }

        for (&key, plateau) in &self.plateaus {
            let elems: Vec<_> = self
                .plateau_basis
                .basis_elements(&key)
                .iter()
                .map(|&gid| {
                    let g = gnodes.get(gid.index());
                    (
                        gid.index(),
                        g.sum().to_f64_approx(),
                        format!("{:?}", g.state()),
                    )
                })
                .collect();
            let expected: f64 = elems.iter().map(|e| e.1).sum();
            let actual = plateau.sum.to_f64_approx();
            assert!(
                expected == actual || (expected - actual).abs() < 1e-9,
                "{label}: plateau sum mismatch at key {key:?}\n\
                 tracked={actual}, recomputed={expected}\n\
                 basis_elements={elems:?}",
            );
        }
    }
}
