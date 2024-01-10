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
    pub const fn new() -> Self {
        Self {
            nodes: PrimalNodes::new(),
        }
    }
}

impl<const N: usize, const DOUBLE_N: usize> PrimalInterface for PrimalModuleEmbedded<N, DOUBLE_N> {
    fn reset(&mut self) {
        self.nodes.clear();
    }

    fn is_blossom(&self, node_index: CompactNodeIndex) -> bool {
        self.nodes.is_blossom(node_index)
    }

    /// query the structure of a blossom with detailed information of touching points
    #[inline]
    fn iterate_blossom_children(
        &self,
        blossom_index: CompactNodeIndex,
        mut func: impl FnMut(&Self, CompactNodeIndex, &TouchingLink),
    ) {
        self.nodes
            .iterate_blossom_children(blossom_index, |node_index, link| func(self, node_index, link));
    }

    /// resolve one obstacle
    #[allow(unused_mut)]
    fn resolve(&mut self, dual_module: &mut impl DualInterface, obstacle: CompactObstacle) -> bool {
        debug_assert!(obstacle.is_obstacle());
        match obstacle {
            CompactObstacle::Conflict {
                node_1,
                mut node_2,
                touch_1,
                touch_2,
                vertex_1,
                vertex_2,
            } => {
                debug_assert!(node_1 != node_2, "one cannot conflict with itself");
                debug_assert!(node_1.is_some() && touch_1.is_some());
                let mut node_1 = usu!(node_1);
                let touch_1 = usu!(touch_1);
                self.nodes.check_node_index(node_1);
                self.nodes.check_node_index(touch_1);
                cfg_if::cfg_if! {
                    if #[cfg(feature="obstacle_potentially_outdated")] {
                        if self.nodes.is_blossom(node_1) && !self.nodes.has_node(node_1) {
                            return true; // outdated event
                        }
                        // also convert the conflict to between the outer blossom
                        node_1 = self.nodes.get_outer_blossom(node_1);
                        if let Some(some_node_2) = node_2.option() {
                            self.nodes.check_node_index(some_node_2);
                            self.nodes.check_node_index(usu!(touch_2));
                            if self.nodes.is_blossom(some_node_2) && !self.nodes.has_node(some_node_2) {
                                return true; // outdated event
                            }
                            node_2 = self.nodes.get_outer_blossom(some_node_2).option();
                        }
                    }
                }
                debug_assert!(
                    self.nodes.get_outer_blossom(node_1) == node_1,
                    "outdated event found but feature not enabled"
                );
                if let Some(node_2) = node_2.option() {
                    self.nodes.check_node_index(node_2);
                    self.nodes.check_node_index(usu!(touch_2));
                    debug_assert!(
                        self.nodes.get_outer_blossom(node_2) == node_2,
                        "outdated event found but feature not enabled"
                    );
                    cfg_if::cfg_if! { if #[cfg(feature="obstacle_potentially_outdated")] {
                        if node_1 == node_2 {
                            return true; // outdated event: already in the same blossom
                        }
                        if !CompactGrowState::is_conflicting(
                                self.nodes.get_grow_state(node_1), self.nodes.get_grow_state(node_2)) {
                            return true; // outdated event
                        }
                    } }
                    self.resolve_conflict(dual_module, node_1, node_2, touch_1, usu!(touch_2), vertex_1, vertex_2)
                } else {
                    cfg_if::cfg_if! { if #[cfg(feature="obstacle_potentially_outdated")] {
                        if self.nodes.get_grow_state(node_1) != CompactGrowState::Grow {
                            return true; // outdated event
                        }
                    } }
                    self.resolve_conflict_virtual(dual_module, node_1, touch_1, vertex_1, vertex_2)
                }
            }
            CompactObstacle::BlossomNeedExpand { mut blossom } => {
                debug_assert!(self.nodes.is_blossom(blossom));
                cfg_if::cfg_if! { if #[cfg(feature="obstacle_potentially_outdated")] {
                    if !self.nodes.has_node(blossom) {
                        return true; // outdated event
                    }
                    // also convert the event to the outer blossom
                    blossom = self.nodes.get_outer_blossom(blossom);
                    if self.nodes.get_grow_state(blossom) != CompactGrowState::Shrink {
                        return true; // outdated event
                    }
                } }
                self.resolve_blossom_need_expand(dual_module, blossom)
            }
            _ => unimplemented_or_loop!(),
        }
    }

    #[inline]
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
        self.nodes.set_speed(node_index, CompactGrowState::Grow, dual_module);
        true
    }
}

impl<const N: usize, const DOUBLE_N: usize> PrimalModuleEmbedded<N, DOUBLE_N> {
    /// return the perfect matching between nodes
    #[inline]
    pub fn iterate_intermediate_matching(
        &mut self,
        mut func: impl FnMut(&Self, CompactNodeIndex, CompactMatchTarget, &TouchingLink),
    ) {
        self.nodes
            .iterate_intermediate_matching(|node_index, match_target, link| func(self, node_index, match_target, link));
    }

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
    ) -> bool {
        let primal_node_1 = self.nodes.get_node(node_1);
        let primal_node_2 = self.nodes.get_node(node_2);
        // this is the most probable case, so put it in the front
        let free_1 = primal_node_1.is_free();
        let free_2 = primal_node_2.is_free();
        if free_1 && free_2 {
            // simply match them temporarily
            self.nodes
                .temporary_match(dual_module, node_1, node_2, touch_1, touch_2, vertex_1, vertex_2);
            return true;
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
                    self.nodes.set_speed(free_node, CompactGrowState::Grow, dual_module);
                    self.nodes.set_speed(matched_node, CompactGrowState::Shrink, dual_module);
                    self.nodes.set_speed(leaf_node, CompactGrowState::Grow, dual_module);
                    let free_primal_node = self.nodes.get_node_mut(free_node);
                    free_primal_node.first_child = matched_node.option();
                    let matched_primal_node = self.nodes.get_node_mut(matched_node);
                    matched_primal_node.parent = free_node.option();
                    matched_primal_node.first_child = leaf_node.option();
                    matched_primal_node.sibling.set_none();
                    matched_primal_node.link.touch = matched_touch.option();
                    matched_primal_node.link.through = matched_vertex.option();
                    matched_primal_node.link.peer_touch = free_touch.option();
                    matched_primal_node.link.peer_through = free_vertex.option();
                    let leaf_primal_node = self.nodes.get_node_mut(leaf_node);
                    leaf_primal_node.parent = matched_node.option();
                    leaf_primal_node.sibling.set_none();
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
            return true;
        }
        // Bambu HLS cannot handle recursive functions for now (without an internal stack), which means
        // the HLS primal module has to stop here, and report `false` to indicate it's not handled
        cfg_if::cfg_if! {
            if #[cfg(feature="hls")] {
                return false;
            }
        }
        // third probable case: tree touches single vertex
        let in_alternating_tree_1 = primal_node_1.in_alternating_tree();
        let in_alternating_tree_2 = primal_node_2.in_alternating_tree();
        if (free_1 && in_alternating_tree_2) || (free_2 && in_alternating_tree_1) {
            let in_tree_node = if free_1 { node_2 } else { node_1 };
            self.augment_whole_tree(dual_module, in_tree_node);
            self.nodes
                .temporary_match(dual_module, node_1, node_2, touch_1, touch_2, vertex_1, vertex_2);
            return true;
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
                    self.nodes.set_speed(matched_node, CompactGrowState::Shrink, dual_module);
                    self.nodes.set_speed(matched_peer_node, CompactGrowState::Grow, dual_module);
                    // set the matched peer to leaf
                    let matched_peer_primal_node = self.nodes.get_node_mut(matched_peer_node);
                    matched_peer_primal_node.parent = matched_node.option();
                    debug_assert_eq!(matched_peer_primal_node.sibling, matched_node.option(), "before breaking");
                    matched_peer_primal_node.sibling.set_none();
                    // set the matched node as the first child of the in-tree node
                    let in_tree_primal_node = self.nodes.get_node_mut(in_tree_node);
                    let first_child = in_tree_primal_node.first_child;
                    in_tree_primal_node.first_child = matched_node.option();
                    // set the parent of the matched node as the in-tree node
                    let matched_primal_node = self.nodes.get_node_mut(matched_node);
                    matched_primal_node.parent = in_tree_node.option();
                    matched_primal_node.sibling = first_child;
                    matched_primal_node.first_child = matched_peer_node.option();
                    matched_primal_node.link.touch = matched_touch.option();
                    matched_primal_node.link.through = matched_vertex.option();
                    matched_primal_node.link.peer_touch = in_tree_touch.option();
                    matched_primal_node.link.peer_through = in_tree_vertex.option();
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
            return true;
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
                self.augment_whole_tree(dual_module, node_1);
                self.augment_whole_tree(dual_module, node_2);
                self.nodes
                    .temporary_match(dual_module, node_1, node_2, touch_1, touch_2, vertex_1, vertex_2);
            }
            return true;
        }
        unreachable!();
    }

    /// handle an up-to-date conflict virtual event
    pub fn resolve_conflict_virtual(
        &mut self,
        dual_module: &mut impl DualInterface,
        node: CompactNodeIndex,
        touch: CompactNodeIndex,
        vertex: CompactVertexIndex,
        virtual_vertex: CompactVertexIndex,
    ) -> bool {
        let primal_node = self.nodes.get_node(node);
        if primal_node.in_alternating_tree() {
            self.augment_whole_tree(dual_module, node);
        }
        self.nodes
            .temporary_match_virtual_vertex(dual_module, node, touch, vertex, virtual_vertex);
        true
    }

    /// handle an up-to-date blossom need expand event
    pub fn resolve_blossom_need_expand(&mut self, dual_module: &mut impl DualInterface, blossom: CompactNodeIndex) -> bool {
        dual_module.expand_blossom(self, blossom);
        // the blossom is guaranteed to be a - node in the alternating tree, thus only 1 children
        let blossom_primal_node = self.nodes.get_node(blossom);
        let parent_index = usu!(blossom_primal_node.parent);
        let child_index = usu!(blossom_primal_node.first_child);
        let touch_to_parent = usu!(blossom_primal_node.link.touch);
        let touch_to_child = usu!(self.nodes.get_node(child_index).link.peer_touch);
        let inner_to_parent = self.nodes.get_second_outer_blossom(touch_to_parent);
        let inner_to_child = self.nodes.get_second_outer_blossom(touch_to_child);
        // find the index of the inner nodes in the odd cycle
        let mut cycle_index_parent = None;
        let mut cycle_index_child = None;
        let first_blossom_child = self.nodes.get_first_blossom_child(blossom);
        if first_blossom_child == inner_to_parent {
            cycle_index_parent = Some(0);
        }
        if first_blossom_child == inner_to_child {
            cycle_index_child = Some(0);
        }
        let mut inner_node = usu!(self.nodes.get_node(first_blossom_child).sibling);
        let mut cycle_index = 1;
        while inner_node != first_blossom_child {
            if inner_node == inner_to_parent {
                cycle_index_parent = Some(cycle_index);
            }
            if inner_node == inner_to_child {
                cycle_index_child = Some(cycle_index);
            }
            inner_node = usu!(self.nodes.get_node(inner_node).sibling);
            cycle_index += 1;
        }
        debug_assert!(cycle_index % 2 == 1, "should be an odd cycle");
        let cycle_length = cycle_index;
        let cycle_index_parent = usu!(cycle_index_parent);
        let cycle_index_child = usu!(cycle_index_child);
        // there are two paths from the start to the end in the cycle: one is odd and the other is even
        // we will match the even path internally, and then attach the odd path in the alternating tree
        // note that in the special case where inner nodes are equal, then all the other nodes are matched internally
        // we don't need special logic for that because all the other nodes will constitute an even path
        let clockwise_distance = if cycle_index_child >= cycle_index_parent {
            cycle_index_child - cycle_index_parent
        } else {
            (cycle_length + cycle_index_child) - cycle_index_parent
        };
        let blossom_primal_node = self.nodes.get_node(blossom);
        let to_parent_link = blossom_primal_node.link.clone();
        if clockwise_distance % 2 == 0 {
            // attach clockwise path to the alternating tree
            self.expand_blossom_match_chain(
                dual_module,
                usu!(self.nodes.get_node(inner_to_child).sibling),
                inner_to_parent,
            ); // first match the clockwise even chain from inner child to inner parent
            let primal_inner_to_parent = self.nodes.get_node_mut(inner_to_parent);
            let mut last_link = primal_inner_to_parent.link.clone();
            let mut next_node = usu!(primal_inner_to_parent.sibling);
            primal_inner_to_parent.link = to_parent_link;
            primal_inner_to_parent.parent = parent_index.option();
            primal_inner_to_parent.first_child = next_node.option();
            self.nodes.set_speed(inner_to_parent, CompactGrowState::Shrink, dual_module);
            // go along the odd path
            let mut node = inner_to_parent;
            let mut is_growing = true;
            while node != inner_to_child {
                self.nodes.set_speed(
                    next_node,
                    if is_growing {
                        CompactGrowState::Grow
                    } else {
                        CompactGrowState::Shrink
                    },
                    dual_module,
                );
                let primal_next_node = self.nodes.get_node_mut(next_node);
                let previous_link = primal_next_node.link.clone();
                primal_next_node.link.touch = last_link.peer_touch;
                primal_next_node.link.through = last_link.peer_through;
                primal_next_node.link.peer_touch = last_link.touch;
                primal_next_node.link.peer_through = last_link.through;
                // alternating grow and shrink
                primal_next_node.parent = node.option();
                // it is wrong for the last node, so need to recover later
                let previous_sibling = usu!(primal_next_node.sibling);
                primal_next_node.first_child = previous_sibling.option();
                primal_next_node.sibling.set_none();
                last_link = previous_link;
                node = next_node;
                next_node = previous_sibling;
                is_growing = !is_growing;
            }
            // fix the tail
            let primal_inner_to_child = self.nodes.get_node_mut(inner_to_child);
            primal_inner_to_child.first_child = child_index.option();
            // internally match the remaining
        } else {
            // attach counter-clockwise path to the alternating tree
            self.expand_blossom_match_chain(
                dual_module,
                usu!(self.nodes.get_node(inner_to_parent).sibling),
                inner_to_child,
            ); // first match the clockwise even chain from inner parent to inner child
            let mut node = inner_to_child;
            let mut first_child = child_index;
            let mut is_growing = false;
            loop {
                self.nodes.set_speed(
                    node,
                    if is_growing {
                        CompactGrowState::Grow
                    } else {
                        CompactGrowState::Shrink
                    },
                    dual_module,
                );
                let primal_node = self.nodes.get_node_mut(node);
                let next_node = usu!(primal_node.sibling);
                primal_node.parent = next_node.option(); // it is wrong for the last node, so need to recover later
                primal_node.sibling.set_none();
                primal_node.first_child = first_child.option();
                if node == inner_to_parent {
                    break;
                }
                first_child = node;
                node = next_node;
                is_growing = !is_growing;
            }
            let primal_inner_to_parent = self.nodes.get_node_mut(inner_to_parent);
            primal_inner_to_parent.link = to_parent_link;
            primal_inner_to_parent.parent = parent_index.option();
        }
        // fix the parent
        debug_assert!(
            self.nodes.get_node(parent_index).sibling.is_none(),
            "+ node should not have any sibling"
        );
        self.alternating_tree_replace_child_with(parent_index, blossom, inner_to_parent);
        // fix the child
        let primal_child = self.nodes.get_node_mut(child_index);
        primal_child.parent = inner_to_child.option();
        debug_assert!(primal_child.sibling.is_none(), "+ node should not have any sibling");
        // remove the blossom
        self.nodes.dispose_blossom(blossom);
        true
    }

    #[inline]
    fn expand_blossom_match_chain(
        &mut self,
        dual_module: &mut impl DualInterface,
        begin: CompactNodeIndex,
        end: CompactNodeIndex,
    ) {
        let mut matching = begin;
        while matching != end {
            self.nodes.set_speed(matching, CompactGrowState::Stay, dual_module);
            let primal_matching = self.nodes.get_node_mut(matching);
            let link = primal_matching.link.clone();
            primal_matching.parent.set_none();
            primal_matching.first_child.set_none();
            let peer = usu!(primal_matching.sibling);
            debug_assert!(peer != end, "should not be an odd chain");
            self.nodes.set_speed(peer, CompactGrowState::Stay, dual_module);
            let primal_peer = self.nodes.get_node_mut(peer);
            primal_peer.link.touch = link.peer_touch;
            primal_peer.link.through = link.peer_through;
            primal_peer.link.peer_touch = link.touch;
            primal_peer.link.peer_through = link.through;
            primal_peer.parent.set_none();
            primal_peer.first_child.set_none();
            let next_matching = usu!(primal_peer.sibling);
            primal_peer.sibling = matching.option();
            matching = next_matching;
        }
    }

    #[inline]
    fn alternating_tree_replace_child_with(&mut self, node: CompactNodeIndex, from: CompactNodeIndex, to: CompactNodeIndex) {
        let primal_node = self.nodes.get_node_mut(node);
        if usu!(primal_node.first_child) == from {
            primal_node.first_child = to.option();
        } else {
            let mut node = usu!(primal_node.first_child);
            loop {
                let primal_node = self.nodes.get_node(node);
                if primal_node.sibling == from.option() {
                    break;
                }
                debug_assert!(primal_node.sibling.is_some(), "cannot find the blossom in the child list");
                node = usu!(primal_node.sibling);
            }
            let primal_node = self.nodes.get_node_mut(node);
            primal_node.sibling = to.option();
        }
        self.nodes.get_node_mut(to).sibling = self.nodes.get_node(from).sibling;
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
        if let Some(parent_node) = tree_primal_node.parent.option() {
            debug_assert!(
                self.nodes.get_grow_state(parent_node) == CompactGrowState::Shrink,
                "must be - node"
            );
            let parent_primal_node = self.nodes.get_node(parent_node);
            debug_assert!(parent_primal_node.parent.is_some(), "- node should always have parent");
            let ancestor_node = usu!(parent_primal_node.parent);
            debug_assert!(self.nodes.get_node(ancestor_node).is_outer_blossom());
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
        let root_primal_node = self.nodes.get_node_mut(root_node);
        let mut first_child = root_primal_node.first_child;
        root_primal_node.first_child.set_none();
        // expand the subtree of its children
        while let Some(first_child_node) = first_child.option() {
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
        while let Some(first_grandchild_node) = first_grandchild.option() {
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
            if let Some(parent) = node.parent.option() {
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
        if let Some(lca_parent) = lca_parent.option() {
            self.alternating_tree_replace_child_with(lca_parent, lca, blossom);
        }
        // walk from node_1/2 upwards to the LCA and attach all children to the blossom
        self.blossom_construction_transfer_two_paths_to_blossom(node_1, node_2, lca, blossom);
        // connect the two paths in an odd cycle
        let mut iter_1 = node_1;
        while iter_1 != lca {
            let node = self.nodes.get_node_mut(iter_1);
            iter_1 = usu!(node.parent);
            node.sibling = node.parent;
            node.parent = blossom.option();
            node.first_child.set_none();
            node.grow_state = None; // mark as inside a blossom
        }
        let mut iter_2 = node_2;
        let mut last_link = TouchingLink {
            touch: touch_1.option(),
            through: vertex_1.option(),
            peer_touch: touch_2.option(),
            peer_through: vertex_2.option(),
        };
        let mut last_node = node_1.option();
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
            last_node = iter_2.option();
            let original_parent = node.parent;
            node.parent = blossom.option();
            node.first_child.set_none();
            node.grow_state = None;
            if iter_2 == lca {
                break;
            }
            iter_2 = usu!(original_parent);
        }
        dual_module.create_blossom(self, blossom);
    }

    #[inline]
    fn blossom_construction_transfer_two_paths_to_blossom(
        &mut self,
        node_1: CompactNodeIndex,
        node_2: CompactNodeIndex,
        lca: CompactNodeIndex,
        blossom: CompactNodeIndex,
    ) {
        debug_assert!(self.nodes.get_node(lca).grow_state == Some(CompactGrowState::Grow), "+ node");
        debug_assert!(self.nodes.get_node(node_1).grow_state == Some(CompactGrowState::Grow), "+");
        debug_assert!(self.nodes.get_node(node_2).grow_state == Some(CompactGrowState::Grow), "+");
        let path_1_second = self.blossom_construction_transfer_path_children_to_blossom(node_1, lca, blossom);
        let path_2_second = self.blossom_construction_transfer_path_children_to_blossom(node_2, lca, blossom);
        // also transfer all children of lca, except for those two paths
        self.blossom_construction_transfer_children_except_for_two(lca, path_1_second, path_2_second, blossom);
    }

    #[inline]
    fn blossom_construction_transfer_path_children_to_blossom(
        &mut self,
        mut node: CompactNodeIndex,
        lca: CompactNodeIndex,
        blossom: CompactNodeIndex,
    ) -> CompactNodeIndex {
        let mut previous = node;
        while node != lca {
            self.blossom_construction_transfer_children_except_for(node, previous, blossom);
            previous = node;
            debug_assert!(self.nodes.get_node(node).parent.is_some(), "cannot find lca on the way up");
            node = usu!(self.nodes.get_node(node).parent);
        }
        previous
    }

    #[inline]
    fn blossom_construction_transfer_to(
        &mut self,
        child_index: CompactNodeIndex,
        blossom: CompactNodeIndex,
    ) -> OptionCompactNodeIndex {
        let primal_blossom = self.nodes.get_node_mut(blossom);
        let previous_first_child = primal_blossom.first_child;
        primal_blossom.first_child = child_index.option();
        let primal_child = self.nodes.get_node_mut(child_index);
        primal_child.parent = blossom.option();
        let child = primal_child.sibling;
        primal_child.sibling = previous_first_child;
        child
    }

    #[inline]
    fn blossom_construction_transfer_children_except_for(
        &mut self,
        node: CompactNodeIndex,
        except: CompactNodeIndex,
        blossom: CompactNodeIndex,
    ) {
        let mut child = self.nodes.get_node(node).first_child;
        while let Some(child_index) = child.option() {
            if child_index != except {
                debug_assert_eq!(self.nodes.get_node(child_index).parent.option(), Some(node));
                child = self.blossom_construction_transfer_to(child_index, blossom);
            } else {
                child = self.nodes.get_node(child_index).sibling;
            }
        }
    }

    #[inline]
    fn blossom_construction_transfer_children_except_for_two(
        &mut self,
        node: CompactNodeIndex,
        except_1: CompactNodeIndex,
        except_2: CompactNodeIndex,
        blossom: CompactNodeIndex,
    ) {
        let mut child = self.nodes.get_node(node).first_child;
        while let Some(child_index) = child.option() {
            if child_index != except_1 && child_index != except_2 {
                debug_assert_eq!(self.nodes.get_node(child_index).parent.option(), Some(node));
                child = self.blossom_construction_transfer_to(child_index, blossom);
            } else {
                child = self.nodes.get_node(child_index).sibling;
            }
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
        // cargo test --features u16_index primal_module_embedded_size -- --nocapture
        const N: usize = 1000000;
        const DOUBLE_N: usize = 2 * N;
        let total_size = core::mem::size_of::<PrimalModuleEmbedded<N, DOUBLE_N>>();
        println!("memory: {} bytes per node", total_size / DOUBLE_N);
        println!("memory overhead: {} bytes", total_size - (total_size / DOUBLE_N) * DOUBLE_N);
        cfg_if::cfg_if! {
            if #[cfg(feature="u16_index")] {
                assert_eq!(total_size / DOUBLE_N, 16 + 1);
                assert_eq!(core::mem::size_of::<Option<PrimalNode>>(), 16);
            } else {
                assert_eq!(total_size / DOUBLE_N, 2 * (16 + 1));
                assert_eq!(core::mem::size_of::<Option<PrimalNode>>(), 2 * 16);
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
