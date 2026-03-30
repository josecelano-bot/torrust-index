use super::DynamicPlateauTracker;
use crate::tree::gtree::GNodeTree;
use crate::handle::GNodeId;
use crate::traits::{Accumulator, Coordinate};

impl<C: Coordinate, V: Accumulator> DynamicPlateauTracker<C, V> {
    pub(super) fn on_observe_impl(&mut self, gnodes: &GNodeTree<C, V>, g_id: GNodeId, value: V) {
        let mut cur = Some(g_id);
        while let Some(id) = cur {
            if let Some(key) = self.plateau_basis.plateau_key(id) {
                if let Some(p) = self.plateaus.get_mut(&key) {
                    p.sum = V::add(p.sum, value);
                }
            }
            cur = gnodes.get(id.index()).parent();
        }

        #[cfg(debug_assertions)]
        {
            let mut cur = Some(g_id);
            while let Some(id) = cur {
                if let Some(key) = self.plateau_basis.plateau_key(id) {
                    let elems: Vec<_> = self
                        .plateau_basis
                        .basis_elements(&key)
                        .iter()
                        .map(|&r| {
                            let g = gnodes.get(r.index());
                            (
                                r.index(),
                                format!("{:?}", g.sum()),
                                format!("{:?}", g.state()),
                            )
                        })
                        .collect();
                    let expected: V = self
                        .plateau_basis
                        .basis_elements(&key)
                        .iter()
                        .map(|&r| gnodes.get(r.index()).sum())
                        .fold(V::zero(), V::add);
                    assert_eq!(
                        self.plateaus[&key].sum,
                        expected,
                        "POST-OBSERVE: plateau sum drift at key {key:?}\n\
                         delta={:?}, g_id=G({}), cur_node=G({})\n\
                         basis_elements={elems:?}",
                        value,
                        g_id.index(),
                        id.index(),
                    );
                }
                cur = gnodes.get(id.index()).parent();
            }
        }
    }
}
