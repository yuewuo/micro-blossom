//! Primal Module Embedded
//!
//! An embedded implementation of the primal module that assumes a maximum number of nodes;
//! Also, thanks to primal offloading, a primal module doesn't have to remember all the defect nodes.
//! It only initializes a defect node when it's encountered.
//! Besides, the primal module specifies the index of the blossom, which usually starts from an address
//! that is guaranteed to be distinguishable from defect vertices.
//! Only in this way, we can safely use primal offloading without worrying about mixing with a created blossom.
//!

use crate::interface::*;
use crate::primal_nodes::*;
use crate::util::*;

#[cfg_attr(any(test, feature = "std"), derive(Debug))]
pub struct PrimalModuleEmbedded<const N: usize, const DOUBLE_N: usize> {
    /// the alternating tree nodes
    pub nodes: PrimalNodes<N, DOUBLE_N>,
}

impl<const N: usize, const DOUBLE_N: usize> PrimalModuleEmbedded<N, DOUBLE_N> {
    pub fn new() -> Self {
        Self {
            nodes: PrimalNodes::new(),
        }
    }
}

impl<const N: usize, const DOUBLE_N: usize> PrimalInterface for PrimalModuleEmbedded<N, DOUBLE_N> {
    fn clear(&mut self) {
        self.nodes.clear();
    }

    fn is_blossom(&self, node_index: CompactNodeIndex) -> bool {
        self.nodes.is_blossom(node_index)
    }

    /// query the structure of a blossom
    fn iterate_blossom_children(&self, blossom_index: CompactNodeIndex, mut func: impl FnMut(&Self, CompactNodeIndex)) {
        self.nodes
            .iterate_blossom_children(blossom_index, |node_index| func(self, node_index));
    }

    /// query the structure of a blossom with detailed information of touching points
    fn iterate_blossom_children_with_touching(
        &self,
        blossom_index: CompactNodeIndex,
        mut func: impl FnMut(
            &Self,
            CompactNodeIndex,
            ((CompactNodeIndex, CompactVertexIndex), (CompactNodeIndex, CompactVertexIndex)),
        ),
    ) {
        self.nodes
            .iterate_blossom_children_with_touching(blossom_index, |node_index, touching_info| {
                func(self, node_index, touching_info)
            });
    }

    /// resolve one obstacle
    #[allow(unused_mut)]
    fn resolve(&mut self, dual_module: &mut impl DualInterface, obstacle: MaxUpdateLength) {
        debug_assert!(obstacle.is_obstacle());
        match obstacle {
            MaxUpdateLength::Conflict {
                mut node_1,
                mut node_2,
                touch_1,
                touch_2,
                vertex_1,
                vertex_2,
            } => {
                debug_assert!(Some(node_1) != node_2, "one cannot conflict with itself");
                self.nodes.check_node_index(node_1);
                self.nodes.check_node_index(touch_1);
                cfg_if::cfg_if! {
                    if #[cfg(feature="obstacle_potentially_outdated")] {
                        if self.nodes.is_blossom(node_1) && !self.nodes.has_node(node_1) {
                            return; // outdated event
                        }
                        // also convert the conflict to between the outer blossom
                        node_1 = self.nodes.get_outer_blossom(node_1);
                        if let Some(some_node_2) = node_2 {
                            self.nodes.check_node_index(some_node_2);
                            self.nodes.check_node_index(usu!(touch_2));
                            if self.nodes.is_blossom(some_node_2) && !self.nodes.has_node(some_node_2) {
                                return; // outdated event
                            }
                            node_2 = Some(self.nodes.get_outer_blossom(some_node_2));
                        }
                    }
                }
                debug_assert!(
                    self.nodes.get_outer_blossom(node_1) == node_1,
                    "outdated event found but feature not enabled"
                );
                if let Some(node_2) = node_2 {
                    self.nodes.check_node_index(node_2);
                    self.nodes.check_node_index(usu!(touch_2));
                    debug_assert!(
                        self.nodes.get_outer_blossom(node_2) == node_2,
                        "outdated event found but feature not enabled"
                    );
                    cfg_if::cfg_if! { if #[cfg(feature="obstacle_potentially_outdated")] {
                        if node_1 == node_2 {
                            return; // outdated event: already in the same blossom
                        }
                        if !CompactGrowState::is_conflicting(
                                self.nodes.get_grow_state(node_1), self.nodes.get_grow_state(node_2)) {
                            return; // outdated event
                        }
                    } }
                    self.resolve_conflict(dual_module, node_1, node_2, touch_1, usu!(touch_2), vertex_1, vertex_2);
                } else {
                    cfg_if::cfg_if! { if #[cfg(feature="obstacle_potentially_outdated")] {
                        if self.nodes.get_grow_state(node_1) != CompactGrowState::Grow {
                            return; // outdated event
                        }
                    } }
                    self.resolve_conflict_virtual(dual_module, node_1, touch_1, vertex_1, vertex_2);
                }
            }
            MaxUpdateLength::BlossomNeedExpand { blossom } => {
                cfg_if::cfg_if! { if #[cfg(feature="obstacle_potentially_outdated")] {
                    if self.nodes.get_grow_state(blossom) != CompactGrowState::Shrink {
                        return; // outdated event
                    }
                } }
                self.resolve_blossom_need_expand(dual_module, blossom);
            }
            _ => unimplemented!(),
        }
    }

    /// return the perfect matching between nodes
    fn iterate_perfect_matching(
        &mut self,
        mut func: impl FnMut(&Self, CompactNodeIndex, CompactMatchTarget, &TouchingLink),
    ) {
        self.nodes
            .iterate_perfect_matching(|node_index, match_target, link| func(self, node_index, match_target, link));
    }

    fn break_with_virtual_vertex(
        &mut self,
        dual_module: &mut impl DualInterface,
        virtual_vertex: CompactVertexIndex,
        hint_node_index: CompactNodeIndex,
    ) -> bool {
        self.nodes.check_node_index(hint_node_index);
        if self.nodes.is_blossom(hint_node_index) && !self.nodes.has_node(hint_node_index) {
            return false; // outdated event, no need to break with virtual vertex anymore
        }
        let node_index = self.nodes.get_outer_blossom(hint_node_index);
        let node = self.nodes.get_node_mut(node_index);
        if !node.is_matched() {
            return false;
        }
        if node.get_matched() == CompactMatchTarget::VirtualVertex(virtual_vertex) {
            return false;
        }
        node.remove_from_matching();
        self.nodes.set_grow_state(node_index, CompactGrowState::Grow, dual_module);
        true
    }
}

impl<const N: usize, const DOUBLE_N: usize> PrimalModuleEmbedded<N, DOUBLE_N> {
    /// handle an up-to-date conflict event
    pub fn resolve_conflict(
        &mut self,
        dual_module: &mut impl DualInterface,
        node_1: CompactNodeIndex,
        node_2: CompactNodeIndex,
        touch_1: CompactNodeIndex,
        touch_2: CompactNodeIndex,
        vertex_1: CompactVertexIndex,
        vertex_2: CompactVertexIndex,
    ) {
        let primal_node_1 = self.nodes.get_node(node_1);
        let primal_node_2 = self.nodes.get_node(node_2);
        // this is the most probable case, so put it in the front
        let free_1 = primal_node_1.is_free();
        let free_2 = primal_node_2.is_free();
        if free_1 && free_2 {
            // simply match them temporarily
            self.nodes
                .temporary_match(dual_module, node_1, node_2, touch_1, touch_2, vertex_1, vertex_2);
            return;
        }
        // second probable case: single node touches a temporary matched pair and become an alternating tree
        if (free_1 && primal_node_2.is_matched()) || (free_2 && primal_node_1.is_matched()) {
            let (free_node, free_touch, free_vertex, matched_node, matched_primal_node, matched_touch, matched_vertex) =
                if free_1 {
                    (node_1, touch_1, vertex_1, node_2, primal_node_2, touch_2, vertex_2)
                } else {
                    (node_2, touch_2, vertex_2, node_1, primal_node_1, touch_1, vertex_1)
                };
            match matched_primal_node.get_matched() {
                CompactMatchTarget::Peer(leaf_node) => {
                    self.nodes.set_grow_state(free_node, CompactGrowState::Grow, dual_module);
                    self.nodes.set_grow_state(matched_node, CompactGrowState::Shrink, dual_module);
                    self.nodes.set_grow_state(leaf_node, CompactGrowState::Grow, dual_module);
                    let free_primal_node = self.nodes.get_node_mut(free_node);
                    free_primal_node.first_child = Some(matched_node);
                    let matched_primal_node = self.nodes.get_node_mut(matched_node);
                    matched_primal_node.parent = Some(free_node);
                    matched_primal_node.first_child = Some(leaf_node);
                    matched_primal_node.sibling = None;
                    matched_primal_node.link.touch = Some(matched_touch);
                    matched_primal_node.link.through = Some(matched_vertex);
                    matched_primal_node.link.peer_touch = Some(free_touch);
                    matched_primal_node.link.peer_through = Some(free_vertex);
                    let leaf_primal_node = self.nodes.get_node_mut(leaf_node);
                    leaf_primal_node.parent = Some(matched_node);
                    leaf_primal_node.sibling = None;
                }
                CompactMatchTarget::VirtualVertex(_virtual_vertex) => {
                    self.nodes.temporary_match(
                        dual_module,
                        free_node,
                        matched_node,
                        free_touch,
                        matched_touch,
                        free_vertex,
                        matched_vertex,
                    );
                }
            }
            return;
        }
        // third probable case: tree touches single vertex
        let in_alternating_tree_1 = primal_node_1.in_alternating_tree();
        let in_alternating_tree_2 = primal_node_2.in_alternating_tree();
        if (free_1 && in_alternating_tree_2) || (free_2 && in_alternating_tree_1) {
            let in_tree_node = if free_1 { node_2 } else { node_1 };
            self.augment_whole_tree(dual_module, in_tree_node);
            self.nodes
                .temporary_match(dual_module, node_1, node_2, touch_1, touch_2, vertex_1, vertex_2);
            return;
        }
        // fourth probable case: tree touches matched pair
        let is_matched_1 = primal_node_1.is_matched();
        let is_matched_2 = primal_node_2.is_matched();
        if (in_alternating_tree_1 && is_matched_2) || (in_alternating_tree_2 && is_matched_1) {
            let (
                matched_node,
                matched_primal_node,
                matched_touch,
                matched_vertex,
                in_tree_node,
                in_tree_touch,
                in_tree_vertex,
            ) = if is_matched_1 {
                (node_1, primal_node_1, touch_1, vertex_1, node_2, touch_2, vertex_2)
            } else {
                (node_2, primal_node_2, touch_2, vertex_2, node_1, touch_1, vertex_1)
            };
            match matched_primal_node.get_matched() {
                CompactMatchTarget::Peer(matched_peer_node) => {
                    self.nodes.set_grow_state(matched_node, CompactGrowState::Shrink, dual_module);
                    self.nodes
                        .set_grow_state(matched_peer_node, CompactGrowState::Grow, dual_module);
                    // set the matched peer to leaf
                    let matched_peer_primal_node = self.nodes.get_node_mut(matched_peer_node);
                    matched_peer_primal_node.parent = Some(matched_node);
                    debug_assert_eq!(matched_peer_primal_node.sibling, Some(matched_node), "before breaking");
                    matched_peer_primal_node.sibling = None;
                    // set the matched node as the first child of the in-tree node
                    let in_tree_primal_node = self.nodes.get_node_mut(in_tree_node);
                    let first_child = in_tree_primal_node.first_child;
                    in_tree_primal_node.first_child = Some(matched_node);
                    // set the parent of the matched node as the in-tree node
                    let matched_primal_node = self.nodes.get_node_mut(matched_node);
                    matched_primal_node.parent = Some(in_tree_node);
                    matched_primal_node.sibling = first_child;
                    matched_primal_node.link.touch = Some(matched_touch);
                    matched_primal_node.link.through = Some(matched_vertex);
                    matched_primal_node.link.peer_touch = Some(in_tree_touch);
                    matched_primal_node.link.peer_through = Some(in_tree_vertex);
                }
                CompactMatchTarget::VirtualVertex(_virtual_vertex) => {
                    // peel the tree
                    self.augment_whole_tree(dual_module, in_tree_node);
                    self.nodes.temporary_match(
                        dual_module,
                        matched_node,
                        in_tree_node,
                        matched_touch,
                        in_tree_touch,
                        matched_vertex,
                        in_tree_vertex,
                    );
                }
            }
            return;
        }
        // much less probable case: two trees touch and both are augmented
        if in_alternating_tree_1 && in_alternating_tree_2 {
            let (root_1, depth_1) = self.alternating_tree_root_of(node_1);
            let (root_2, depth_2) = self.alternating_tree_root_of(node_2);
            if root_1 == root_2 {
                // form a blossom inside an alternating tree
                self.create_blossom_inside_alternating_tree(
                    dual_module,
                    depth_1,
                    depth_2,
                    node_1,
                    node_2,
                    touch_1,
                    touch_2,
                    vertex_1,
                    vertex_2,
                );
            } else {
                // augment the two alternating tree
                unimplemented!();
            }
            return;
        }
        unreachable!()
    }

    /// handle an up-to-date conflict virtual event
    pub fn resolve_conflict_virtual(
        &mut self,
        dual_module: &mut impl DualInterface,
        node: CompactNodeIndex,
        touch: CompactNodeIndex,
        vertex: CompactVertexIndex,
        virtual_vertex: CompactVertexIndex,
    ) {
        let primal_node = self.nodes.get_node(node);
        if primal_node.in_alternating_tree() {
            self.augment_whole_tree(dual_module, node);
        }
        self.nodes
            .temporary_match_virtual_vertex(dual_module, node, touch, vertex, virtual_vertex);
        return;
    }

    /// handle an up-to-date blossom need expand event
    pub fn resolve_blossom_need_expand(&mut self, dual_module: &mut impl DualInterface, blossom: CompactNodeIndex) {
        unimplemented!()
    }

    /// for any + node, match it with another node will augment the whole tree, breaking out into several matched pairs;
    /// this function is called when assuming this node is matched and removed from the alternating tree
    pub fn augment_whole_tree(&mut self, dual_module: &mut impl DualInterface, tree_node: CompactNodeIndex) {
        debug_assert!(
            self.nodes.get_grow_state(tree_node) == CompactGrowState::Grow,
            "must be + node"
        );
        // augment the subtree
        self.augment_subtree(dual_module, tree_node);
        // let the parent match with ancestor, if exists any
        let tree_primal_node = self.nodes.get_node(tree_node);
        if let Some(parent_node) = tree_primal_node.parent {
            debug_assert!(
                self.nodes.get_grow_state(parent_node) == CompactGrowState::Shrink,
                "must be - node"
            );
            let parent_primal_node = self.nodes.get_node(parent_node);
            debug_assert!(parent_primal_node.parent.is_some(), "- node should always have parent");
            let ancestor_node = usu!(parent_primal_node.parent);
            let link = parent_primal_node.link.clone();
            self.augment_whole_tree(dual_module, ancestor_node);
            self.nodes
                .temporary_match_with_link(dual_module, parent_node, &link, ancestor_node);
        }
    }

    /// augment the subtree given a (+) root node, but leave the root node unchanged
    pub fn augment_subtree(&mut self, dual_module: &mut impl DualInterface, root_node: CompactNodeIndex) {
        debug_assert!(
            self.nodes.get_grow_state(root_node) == CompactGrowState::Grow,
            "must be + node"
        );
        let root_primal_node = self.nodes.get_node(root_node);
        let mut first_child = root_primal_node.first_child;

        // expand the subtree of its children
        while let Some(first_child_node) = first_child {
            let child_primal_node = self.nodes.get_node(first_child_node);
            first_child = child_primal_node.sibling;
            self.match_subtree(dual_module, first_child_node);
        }
    }

    /// match the whole subtree given a (-) root node, changing all the subtree including the root node
    pub fn match_subtree(&mut self, dual_module: &mut impl DualInterface, root_node: CompactNodeIndex) {
        debug_assert!(
            self.nodes.get_grow_state(root_node) == CompactGrowState::Shrink,
            "must be - node"
        );
        let root_primal_node = self.nodes.get_node(root_node);
        debug_assert!(
            root_primal_node.first_child.is_some(),
            "- node is always followed by a + node"
        );
        let child_node = usu!(root_primal_node.first_child);
        debug_assert!(
            self.nodes.get_grow_state(child_node) == CompactGrowState::Grow,
            "must be + node"
        );
        let child_primal_node = self.nodes.get_node(child_node);
        debug_assert!(child_primal_node.sibling.is_none(), "+ node should not have any siblings");
        let child_link = child_primal_node.link.clone();
        let mut first_grandchild = child_primal_node.first_child;
        // match root with child
        self.nodes
            .temporary_match_with_link(dual_module, child_node, &child_link, root_node);
        // iterate through the descendants and match the subtrees
        while let Some(first_grandchild_node) = first_grandchild {
            let grandchild_primal_node = self.nodes.get_node(first_grandchild_node);
            first_grandchild = grandchild_primal_node.sibling;
            self.match_subtree(dual_module, first_grandchild_node);
        }
    }

    /// return (the root of the tree, the depth of this node in the tree)
    pub fn alternating_tree_root_of(&self, mut node_index: CompactNodeIndex) -> (CompactNodeIndex, TreeDepth) {
        let mut depth = 0;
        loop {
            let node = self.nodes.get_node(node_index);
            debug_assert!(node.is_outer_blossom());
            if let Some(parent) = node.parent {
                node_index = parent;
                depth += 1;
            } else {
                return (node_index, depth);
            }
        }
    }

    #[inline] // part of the `resolve` function, put it here for code clarity
    fn create_blossom_inside_alternating_tree(
        &mut self,
        dual_module: &mut impl DualInterface,
        depth_1: TreeDepth,
        depth_2: TreeDepth,
        node_1: CompactNodeIndex,
        node_2: CompactNodeIndex,
        touch_1: CompactNodeIndex,
        touch_2: CompactNodeIndex,
        vertex_1: CompactVertexIndex,
        vertex_2: CompactVertexIndex,
    ) {
        debug_assert!(depth_1 % 2 == 0 && depth_2 % 2 == 0, "two nodes must be + node");
        let (lca, depth_lca) = self.find_lowest_common_ancestor(depth_1, depth_2, node_1, node_2);
        debug_assert!(
            self.nodes.get_node(lca).grow_state == Some(CompactGrowState::Grow) && depth_lca % 2 == 0,
            "least common ancestor should always be a + node in the alternating tree"
        );
        // allocate a new blossom
        let blossom = self.nodes.allocate_blossom(lca);
        // swap the lca node with the new blossom in the tree above lca
        let primal_lca = self.nodes.get_node(lca);
        let lca_parent = primal_lca.parent;
        let lca_sibling = primal_lca.sibling;
        let lca_link = primal_lca.link.clone();
        let primal_blossom = self.nodes.get_node_mut(blossom);
        primal_blossom.parent = lca_parent;
        primal_blossom.sibling = lca_sibling;
        primal_blossom.link = lca_link;
        // walk from node_1/2 upwards to the LCA and attach all children to the blossom
        for node in [node_1, node_2] {
            self.blossom_construction_transfer_path_children_to_lca(node, lca);
        }
        // connect the two paths in an odd cycle
        let mut iter_1 = node_1;
        while iter_1 != lca {
            let node = self.nodes.get_node_mut(iter_1);
            iter_1 = usu!(node.parent);
            node.sibling = if iter_1 == lca { None } else { node.parent };
            node.parent = None;
            node.first_child = None;
        }
        let mut iter_2 = node_2;
        let mut last_link = TouchingLink {
            touch: Some(touch_1),
            through: Some(vertex_1),
            peer_touch: Some(touch_2),
            peer_through: Some(vertex_2),
        };
        let mut last_node = if iter_1 == lca { None } else { Some(node_1) };
        loop {
            let node = self.nodes.get_node_mut(iter_2);
            // reverse the link
            let current_link = node.link.clone();
            node.link.touch = last_link.peer_touch;
            node.link.through = last_link.peer_through;
            node.link.peer_touch = last_link.touch;
            node.link.peer_through = last_link.through;
            last_link = current_link;
            // set up sibling
            node.sibling = last_node;
            // update parent
            last_node = Some(iter_2);
            let original_parent = node.parent;
            node.parent = None;
            node.first_child = None;
            if last_node == Some(lca) {
                break;
            }
            iter_2 = usu!(original_parent);
        }
        dual_module.create_blossom(self, blossom);
        self.nodes.set_grow_state(blossom, CompactGrowState::Grow, dual_module);
    }

    #[inline]
    fn blossom_construction_transfer_path_children_to_lca(&mut self, mut node: CompactNodeIndex, lca: CompactNodeIndex) {
        let mut previous = node;
        while node != lca {
            self.blossom_construction_transfer_children_except_for(node, previous, lca);
            previous = node;
            debug_assert!(self.nodes.get_node(node).parent.is_some(), "cannot find lca on the way up");
            node = usu!(self.nodes.get_node(node).parent);
        }
    }

    #[inline]
    fn blossom_construction_transfer_children_except_for(
        &mut self,
        node: CompactNodeIndex,
        except: CompactNodeIndex,
        parent: CompactNodeIndex,
    ) {
        let mut child = self.nodes.get_node(node).first_child;
        while let Some(child_index) = child {
            if child_index != except {
                let primal_parent = self.nodes.get_node_mut(parent);
                let previous_first_child = primal_parent.first_child;
                primal_parent.first_child = Some(child_index);
                let primal_child = self.nodes.get_node_mut(child_index);
                debug_assert_eq!(primal_child.parent, Some(node));
                primal_child.parent = Some(parent);
                primal_child.sibling = previous_first_child;
            }
            child = self.nodes.get_node(child_index).sibling;
        }
    }

    fn find_lowest_common_ancestor(
        &self,
        mut depth_1: TreeDepth,
        mut depth_2: TreeDepth,
        mut node_1: CompactNodeIndex,
        mut node_2: CompactNodeIndex,
    ) -> (CompactNodeIndex, TreeDepth) {
        // first make them the same depth
        match depth_1.cmp(&depth_2) {
            core::cmp::Ordering::Greater => loop {
                let primal_node = self.nodes.get_node(node_1);
                debug_assert!(primal_node.parent.is_some(), "depth is not zero, should have parent");
                node_1 = usu!(primal_node.parent);
                depth_1 -= 1;
                if depth_1 == depth_2 {
                    break;
                }
            },
            core::cmp::Ordering::Less => loop {
                let primal_node = self.nodes.get_node(node_2);
                debug_assert!(primal_node.parent.is_some(), "depth is not zero, should have parent");
                node_2 = usu!(primal_node.parent);
                depth_2 -= 1;
                if depth_1 == depth_2 {
                    break;
                }
            },
            _ => {}
        }
        // now they have the same depth, compare them until they're equal
        debug_assert!(depth_1 == depth_2);
        let mut depth = depth_1;
        loop {
            if node_1 == node_2 {
                return (node_1, depth);
            }
            let primal_node_1 = self.nodes.get_node(node_1);
            let primal_node_2 = self.nodes.get_node(node_2);
            debug_assert!(primal_node_1.parent.is_some(), "cannot find common parent");
            debug_assert!(primal_node_2.parent.is_some(), "cannot find common parent");
            node_1 = usu!(primal_node_1.parent);
            node_2 = usu!(primal_node_2.parent);
            depth -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primal_module_embedded_size() {
        // cargo test primal_module_embedded_size -- --nocapture
        const N: usize = 1000000;
        const DOUBLE_N: usize = 2 * N;
        let total_size = core::mem::size_of::<PrimalModuleEmbedded<N, DOUBLE_N>>();
        println!("memory: {} bytes per node", total_size / DOUBLE_N);
        println!("memory overhead: {} bytes", total_size - (total_size / DOUBLE_N) * DOUBLE_N);
        cfg_if::cfg_if! {
            if #[cfg(feature="u16_index")] {
                assert!(total_size / DOUBLE_N == 16);
            } else {
                assert!(total_size / DOUBLE_N == 32);
            }
        }
    }

    #[test]
    fn primal_module_debug_print() {
        // cargo test primal_module_debug_print -- --nocapture
        const N: usize = 100;
        const DOUBLE_N: usize = 2 * N;
        let mut primal_module: PrimalModuleEmbedded<N, DOUBLE_N> = PrimalModuleEmbedded::new();
        println!("{primal_module:?}");
        primal_module.nodes.check_node_index(ni!(3));
        primal_module.nodes.check_node_index(ni!(1));
        primal_module.nodes.check_node_index(ni!(4));
        println!("{primal_module:?}");
    }
}
