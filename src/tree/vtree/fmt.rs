//! Display helpers for V-node debugging output.
//!
//! `Nd` is a lightweight wrapper around arena references that formats a single
//! node for tracing and panic messages.  It lives here (rather than in
//! `graph/algorithm/fmt.rs`) because it only depends on vtree types and is
//! used by violation-push helpers that also live in this module.
//!
//! `graph/algorithm/fmt.rs` re-exports `Nd` for callers in the graph layer.

use std::fmt;

use crate::traits::Accumulator;
use crate::tree::handle::VNodeId;
use crate::tree::vtree::vnode::VKind;

use super::VNodeTree;

/// Formats a single V-node: `v{idx}(E,{intensity})` or `v{idx}(S{n},{intensity})`.
pub struct Nd<'a, V: Accumulator>(pub &'a VNodeTree<V>, pub VNodeId);

impl<V: Accumulator> fmt::Display for Nd<'_, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let idx = self.1.index();
        if !self.0.is_occupied(idx) {
            return write!(f, "v{idx}(DEAD)");
        }
        let n = self.0.get(idx);
        match &n.kind() {
            VKind::Entry { .. } => write!(f, "v{idx}(E,{:?})", n.intensity()),
            VKind::Structural { children, .. } => {
                write!(f, "v{idx}(S{},{:?})", children.len(), n.intensity())
            }
        }
    }
}
