use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GTree;
use crate::tree::vtree::VTree;

/// The dual-tree core of a `GvGraph`.
///
/// Owns the G-tree (geometric partition skeleton) and the V-tree
/// (intensity-aggregation overlay).  Operations that span both trees belong
/// here; operations that also need `config` or the plateau `tracker` live on
/// `GvGraph` instead.
pub struct GvCore<C: Coordinate, V: Accumulator, const N: u32> {
    pub(crate) gtree: GTree<C, V, N>,
    pub(crate) vtree: VTree<V>,
}

impl<C: Coordinate, V: Accumulator, const N: u32> Clone for GvCore<C, V, N> {
    fn clone(&self) -> Self {
        Self {
            gtree: self.gtree.clone(),
            vtree: self.vtree.clone(),
        }
    }
}
