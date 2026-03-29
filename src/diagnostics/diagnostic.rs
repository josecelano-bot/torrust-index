#![allow(dead_code)]

use crate::arena::Arena;
#[cfg(feature = "dynamic-contour-tracking")]
#[allow(unused_imports)]
pub use crate::diagnostics::plateau_audit::{PlateauAuditContext, audit_plateau_consistency};
use crate::graph::algorithm::rebalance::{self, Ctx};
use crate::handle::VNodeId;
use crate::nodes::vnode::VNode;
use crate::traits::{Accumulator, Inspectable};

mod logging;
use logging::{diagnose_collapse_sibling, log_vtree_ancestry};
mod diagnose;

pub fn audit_violations<V: Accumulator + Inspectable>(
    vnodes: &Arena<VNode<V>>,
    violations: &[VNodeId],
    checkpoint: &str,
) -> Vec<VNodeId> {
    let all_violated = rebalance::find_violated_nodes(vnodes);
    let queued: std::collections::HashSet<usize> = violations.iter().map(|v| v.index()).collect();
    let mut missed = Vec::new();
    for &v in &all_violated {
        if !queued.contains(&v.index()) {
            tracing::error!(
                checkpoint,
                node = %Ctx(vnodes, v),
                "violation NOT in queue",
            );
            missed.push(v);
        }
    }
    missed
}

pub struct MissedViolationContext {
    pub evicted_parent: Option<VNodeId>,
    pub evicted_parent_child_count: usize,
    pub collapse_sibling: Option<VNodeId>,
}

pub fn diagnose_missed_violation<V: Accumulator + Inspectable>(
    vnodes: &Arena<VNode<V>>,
    violated: VNodeId,
    context: &MissedViolationContext,
) {
    diagnose::diagnose_missed_violation_impl(vnodes, violated, context);
}

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
mod tests {
    include!("diagnostic/tests.rs");
}
