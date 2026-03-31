//! Promote operations — V-tree restructuring that increases tree height.
//!
//! All three promote variants are collected here because they share the same
//! structural goal (lifting nodes to reduce violations) while differing in
//! which nodes are moved and whether a new G-node must be created.
//!
//! - [`standard_promote`]: 2-child structural node absorbs into its parent
//!   (pure V-tree, no G-tree mutation).
//! - [`skip_promote`]: entry node skips its parent and joins the grandparent
//!   (pure V-tree, no G-tree mutation).
//! - [`legacy_promote`]: semi-internal G-node expands by allocating a new
//!   G-child, which is the **only** case in the rebalancing path that mutates
//!   the G-tree arena.

use crate::handle::{GNodeId, VNodeId};
use crate::nodes::vnode::{Children, VKind, VNode};
use crate::traits::{Accumulator, Coordinate};
use crate::tree::gtree::GTree;
use crate::tree::vtree::VTree;

use super::rebalance::{Ch, Nd};

pub fn standard_promote<V: Accumulator>(vtree: &mut VTree<V>, c: VNodeId) {
    let p = vtree
        .nodes
        .get(c.index())
        .parent()
        .expect("standard_promote: c must have a parent");
    let _span = tracing::debug_span!(
        "standard_promote",
        c = %Nd(&vtree.nodes, c),
        children = %Ch(&vtree.nodes, c),
    )
    .entered();

    let (c1_id, c1_int, c2_id, c2_int) = {
        let node = vtree.nodes.get(c.index());
        match &node.kind() {
            VKind::Structural { children, .. } => {
                assert!(children.len() == 2, "standard_promote: c must be a 2-node");
                let (id1, int1) = children.get(0);
                let (id2, int2) = children.get(1);
                (id1, int1, id2, int2)
            }
            VKind::Entry { .. } => panic!("standard_promote: c must be structural"),
        }
    };

    let sibling_id = vtree.nodes.sibling_of(p, c);

    let sib_terminal = vtree.nodes.node_has_evictable(sibling_id.0);
    let c1_terminal = vtree.nodes.node_has_evictable(c1_id);
    let c2_terminal = vtree.nodes.node_has_evictable(c2_id);

    let p_node = vtree.nodes.get_mut(p.index());
    if let VKind::Structural {
        children,
        has_evictable,
    } = p_node.kind_mut()
    {
        *children = Children::new_3((c1_id, c1_int), (c2_id, c2_int), sibling_id);
        *has_evictable = c1_terminal || c2_terminal || sib_terminal;
    }

    vtree.nodes.get_mut(c1_id.index()).set_parent(p);
    vtree.nodes.get_mut(c2_id.index()).set_parent(p);

    vtree.nodes.dealloc(c.index());

    vtree.propagate_evictable(p);

    tracing::debug!(result = %Ch(&vtree.nodes, p), "c destroyed, p is 3-node");
}

pub fn skip_promote<V: Accumulator>(vtree: &mut VTree<V>, c: VNodeId) -> Option<VNodeId> {
    let p = vtree
        .nodes
        .get(c.index())
        .parent()
        .expect("skip_promote: c must have a parent");
    let g = vtree
        .nodes
        .get(p.index())
        .parent()
        .expect("skip_promote: p must have a grandparent");
    let _span = tracing::debug_span!(
        "skip_promote",
        c = %Nd(&vtree.nodes, c),
        p = p.index(),
        g = g.index(),
    )
    .entered();

    let (s_id, s_int) = vtree.nodes.sibling_of(p, c);

    let c_int = vtree.nodes.get(c.index()).intensity();

    let (u_id, u_int) = vtree.nodes.sibling_of(g, p);

    let c_terminal = vtree.nodes.node_has_evictable(c);
    let s_terminal = vtree.nodes.node_has_evictable(s_id);
    let u_terminal = vtree.nodes.node_has_evictable(u_id);

    let g_node = vtree.nodes.get_mut(g.index());
    if let VKind::Structural {
        children,
        has_evictable,
    } = g_node.kind_mut()
    {
        *children = Children::new_3((c, c_int), (s_id, s_int), (u_id, u_int));
        *has_evictable = c_terminal || s_terminal || u_terminal;
    }

    vtree.nodes.get_mut(c.index()).set_parent(g);
    vtree.nodes.get_mut(s_id.index()).set_parent(g);

    vtree.nodes.dealloc(p.index());

    vtree.propagate_evictable(g);

    tracing::debug!(result = %Ch(&vtree.nodes, g), "p destroyed, g is 3-node");

    None
}

#[allow(clippy::too_many_lines)]
pub fn legacy_promote<C: Coordinate, V: Accumulator, const N: u32>(
    vtree: &mut VTree<V>,
    gtree: &mut GTree<C, V, N>,
    c: VNodeId,
) -> GNodeId {
    let p = vtree
        .nodes
        .get(c.index())
        .parent()
        .expect("legacy_promote: c must have a parent");
    let g = vtree
        .nodes
        .get(p.index())
        .parent()
        .expect("legacy_promote: p must have a grandparent");

    let gnode_id = match &vtree.nodes.get(c.index()).kind() {
        VKind::Entry { gnode, .. } => *gnode,
        VKind::Structural { .. } => panic!("legacy_promote: c must be an entry"),
    };

    debug_assert!(
        gtree.nodes.get(gnode_id.index()).is_semi_internal(),
        "legacy_promote: backing G-node must be semi-internal"
    );

    let _span = tracing::debug_span!(
        "legacy_promote",
        c = %Nd(&vtree.nodes, c),
        p = p.index(),
        g = g.index(),
        gnode = gnode_id.index(),
    )
    .entered();

    let new_child_id = gtree.nodes.allocate_missing_child(gnode_id);

    let ne = VNode::new_entry(V::zero(), Some(p), new_child_id, true, true);
    let ne_id = VNodeId::from_index(vtree.nodes.alloc(ne).0);
    gtree.nodes.assign_entry(new_child_id, ne_id);

    let c_int = vtree.nodes.get(c.index()).intensity();
    vtree.replace_structural_child(p, c, ne_id, V::zero());

    let (u_id, u_int) = vtree.nodes.sibling_of(g, p);

    let c_evictable = false;
    let p_evictable = vtree.nodes.node_has_evictable(p);
    let u_evictable = vtree.nodes.node_has_evictable(u_id);

    let p_int = vtree.nodes.get(p.index()).intensity();
    let g_node = vtree.nodes.get_mut(g.index());
    if let VKind::Structural {
        children,
        has_evictable,
    } = g_node.kind_mut()
    {
        *children = Children::new_3((c, c_int), (p, p_int), (u_id, u_int));
        *has_evictable = c_evictable || p_evictable || u_evictable;
    }

    vtree.nodes.get_mut(c.index()).set_parent(g);

    vtree.set_entry_flags(c, false, false);

    vtree.recompute_and_sync(p);

    vtree.propagate_evictable(p);
    vtree.propagate_evictable(g);

    tracing::debug!(
        new_gnode = new_child_id.index(),
        new_ventry = ne_id.index(),
        "legacy_promote complete: c lifted to g, new child created",
    );

    new_child_id
}

#[cfg(test)]
mod tests {
    use super::{legacy_promote, skip_promote, standard_promote};
    use crate::arena::Arena;
    use crate::graph::{Config, GvGraph, StructuralConfig};
    use crate::handle::{GNodeId, VNodeId};
    use crate::nodes::vnode::{Children, VKind, VNode};
    use crate::tree::vtree::VTree;

    use crate::tree::vtree::VNodeTree;

    fn make_vtree<V: crate::traits::Accumulator>() -> VTree<V> {
        VTree {
            nodes: VNodeTree::from(Arena::new()),
            violations: Vec::new(),
        }
    }

    fn make_graph() -> GvGraph<u8, u64, 8> {
        GvGraph::new(Config {
            split_threshold: 2,
            structural: StructuralConfig {
                depth_create: 3,
                depth_evict: 5,
                budget: None,
                alpha_relax: 0.5,
                bounded_eviction: false,
            },
        })
    }

    #[test]
    fn standard_promote_replaces_child_pair_with_grandchildren() {
        let mut vtree: VTree<u64> = make_vtree();

        let e1 = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    2,
                    None,
                    GNodeId::from_index(0),
                    false,
                    true,
                ))
                .0,
        );
        let e2 = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    3,
                    None,
                    GNodeId::from_index(1),
                    false,
                    false,
                ))
                .0,
        );
        let s = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    5,
                    None,
                    GNodeId::from_index(2),
                    false,
                    true,
                ))
                .0,
        );

        let c = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_structural(
                    5,
                    None,
                    Children::new_2((e1, 2), (e2, 3)),
                    true,
                ))
                .0,
        );
        vtree.nodes.get_mut(e1.index()).set_parent(c);
        vtree.nodes.get_mut(e2.index()).set_parent(c);

        let p = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_structural(
                    10,
                    None,
                    Children::new_2((c, 5), (s, 5)),
                    true,
                ))
                .0,
        );
        vtree.nodes.get_mut(c.index()).set_parent(p);
        vtree.nodes.get_mut(s.index()).set_parent(p);

        standard_promote(&mut vtree, c);

        let p_node = vtree.nodes.get(p.index());
        match p_node.kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(children.get(0).0, e1);
                assert_eq!(children.get(1).0, e2);
                assert_eq!(children.get(2).0, s);
            }
            VKind::Entry { .. } => panic!("parent should remain structural"),
        }

        assert_eq!(vtree.nodes.get(e1.index()).parent(), Some(p));
        assert_eq!(vtree.nodes.get(e2.index()).parent(), Some(p));
        assert!(!vtree.nodes.is_occupied(c.index()));
    }

    #[test]
    fn skip_promote_lifts_child_and_sibling_to_grandparent() {
        let mut vtree: VTree<u64> = make_vtree();

        let c = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    4,
                    None,
                    GNodeId::from_index(10),
                    false,
                    true,
                ))
                .0,
        );
        let s = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    3,
                    None,
                    GNodeId::from_index(11),
                    false,
                    false,
                ))
                .0,
        );
        let u = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    8,
                    None,
                    GNodeId::from_index(12),
                    false,
                    true,
                ))
                .0,
        );

        let p = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_structural(
                    7,
                    None,
                    Children::new_2((c, 4), (s, 3)),
                    true,
                ))
                .0,
        );
        vtree.nodes.get_mut(c.index()).set_parent(p);
        vtree.nodes.get_mut(s.index()).set_parent(p);

        let g = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_structural(
                    15,
                    None,
                    Children::new_2((p, 7), (u, 8)),
                    true,
                ))
                .0,
        );
        vtree.nodes.get_mut(p.index()).set_parent(g);
        vtree.nodes.get_mut(u.index()).set_parent(g);

        let new_g = skip_promote(&mut vtree, c);
        assert!(new_g.is_none());

        let g_node = vtree.nodes.get(g.index());
        match g_node.kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(children.get(0).0, c);
                assert_eq!(children.get(1).0, s);
                assert_eq!(children.get(2).0, u);
            }
            VKind::Entry { .. } => panic!("grandparent should remain structural"),
        }

        assert_eq!(vtree.nodes.get(c.index()).parent(), Some(g));
        assert_eq!(vtree.nodes.get(s.index()).parent(), Some(g));
        assert!(!vtree.nodes.is_occupied(p.index()));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn legacy_promote_lifts_entry_and_creates_missing_gchild() {
        let mut graph = make_graph();

        let semi_gid = graph.gtree.nodes.root;
        let existing_child = graph.gtree.nodes.allocate_missing_child(semi_gid);
        assert!(graph.gtree.nodes.get(semi_gid.index()).is_semi_internal());

        let c = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_entry(7, None, semi_gid, true, true))
                .0,
        );
        let s = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_entry(
                    5,
                    None,
                    GNodeId::from_index(existing_child.index()),
                    true,
                    true,
                ))
                .0,
        );
        let u = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_entry(
                    11,
                    None,
                    GNodeId::from_index(existing_child.index()),
                    true,
                    true,
                ))
                .0,
        );

        let p = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    12,
                    None,
                    Children::new_2((c, 7), (s, 5)),
                    true,
                ))
                .0,
        );
        graph.vtree.nodes.get_mut(c.index()).set_parent(p);
        graph.vtree.nodes.get_mut(s.index()).set_parent(p);

        let g = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    23,
                    None,
                    Children::new_2((p, 12), (u, 11)),
                    true,
                ))
                .0,
        );
        graph.vtree.nodes.get_mut(p.index()).set_parent(g);
        graph.vtree.nodes.get_mut(u.index()).set_parent(g);

        graph.gtree.nodes.assign_entry(semi_gid, c);

        let new_gid = legacy_promote(&mut graph.vtree, &mut graph.gtree, c);
        let new_entry = graph
            .gtree
            .nodes
            .get(new_gid.index())
            .entry()
            .expect("new G-child must have V-entry");

        let semi = graph.gtree.nodes.get(semi_gid.index());
        assert!(semi.left().is_some());
        assert!(semi.right().is_some());

        match graph.vtree.nodes.get(p.index()).kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(children.get(0).0, new_entry);
                assert_eq!(children.get(1).0, s);
            }
            VKind::Entry { .. } => panic!("p should remain structural"),
        }

        match graph.vtree.nodes.get(g.index()).kind() {
            VKind::Structural { children, .. } => {
                assert_eq!(children.len(), 3);
                assert_eq!(children.get(0).0, c);
                assert_eq!(children.get(1).0, p);
                assert_eq!(children.get(2).0, u);
            }
            VKind::Entry { .. } => panic!("g should remain structural"),
        }

        match graph.vtree.nodes.get(c.index()).kind() {
            VKind::Entry {
                is_exposed,
                is_evictable,
                ..
            } => {
                assert!(!is_exposed);
                assert!(!is_evictable);
            }
            VKind::Structural { .. } => panic!("c should remain entry"),
        }

        match graph.vtree.nodes.get(new_entry.index()).kind() {
            VKind::Entry {
                gnode,
                is_exposed,
                is_evictable,
            } => {
                assert_eq!(*gnode, new_gid);
                assert!(*is_exposed);
                assert!(*is_evictable);
            }
            VKind::Structural { .. } => panic!("new entry must be an entry"),
        }

        assert_eq!(graph.vtree.nodes.get(c.index()).parent(), Some(g));
        assert_eq!(graph.vtree.nodes.get(new_entry.index()).parent(), Some(p));
        assert_eq!(
            graph.gtree.nodes.get(new_gid.index()).parent(),
            Some(semi_gid)
        );
    }

    #[test]
    #[should_panic(expected = "standard_promote: c must be structural")]
    fn standard_promote_panics_for_entry_node() {
        let mut vtree: VTree<u64> = make_vtree();
        let c = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    4,
                    None,
                    GNodeId::from_index(1),
                    true,
                    true,
                ))
                .0,
        );
        let s = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_entry(
                    3,
                    None,
                    GNodeId::from_index(2),
                    true,
                    true,
                ))
                .0,
        );
        let p = VNodeId::from_index(
            vtree
                .nodes
                .alloc(VNode::new_structural(
                    7,
                    None,
                    Children::new_2((c, 4), (s, 3)),
                    true,
                ))
                .0,
        );
        vtree.nodes.get_mut(c.index()).set_parent(p);
        vtree.nodes.get_mut(s.index()).set_parent(p);

        standard_promote(&mut vtree, c);
    }

    #[test]
    #[should_panic(expected = "legacy_promote: c must be an entry")]
    fn legacy_promote_panics_for_structural_c() {
        let mut graph = make_graph();

        let semi_gid = graph.gtree.nodes.root;
        let _ = graph.gtree.nodes.allocate_missing_child(semi_gid);

        let c1 = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_entry(1, None, semi_gid, true, true))
                .0,
        );
        let c2 = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_entry(2, None, semi_gid, true, true))
                .0,
        );
        let c = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    3,
                    None,
                    Children::new_2((c1, 1), (c2, 2)),
                    true,
                ))
                .0,
        );
        graph.vtree.nodes.get_mut(c1.index()).set_parent(c);
        graph.vtree.nodes.get_mut(c2.index()).set_parent(c);

        let u = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_entry(9, None, semi_gid, true, true))
                .0,
        );
        let p = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    3,
                    None,
                    Children::new_2((c, 3), (u, 9)),
                    true,
                ))
                .0,
        );
        graph.vtree.nodes.get_mut(c.index()).set_parent(p);
        graph.vtree.nodes.get_mut(u.index()).set_parent(p);

        let g = VNodeId::from_index(
            graph
                .vtree
                .nodes
                .alloc(VNode::new_structural(
                    12,
                    None,
                    Children::new_2((p, 12), (u, 9)),
                    true,
                ))
                .0,
        );
        graph.vtree.nodes.get_mut(p.index()).set_parent(g);

        let _ = legacy_promote(&mut graph.vtree, &mut graph.gtree, c);
    }
}
