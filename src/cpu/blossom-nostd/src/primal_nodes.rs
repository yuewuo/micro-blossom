//! Primal Nodes
//!
//! A data type that keeps track of (partial) defect nodes and all blossom nodes.
//! The defect nodes are kept partial because of the pre-decoder that never reports some of the
//! defects if they are not evolves in potential complex matchings.
//!

use crate::interface::*;
use crate::util::*;

pub struct PrimalNodes<const N: usize, const DOUBLE_N: usize> {
    /// defect nodes starting from 0, blossom nodes starting from DOUBLE_N/2
    pub buffer: [Option<PrimalNode>; DOUBLE_N],
    /// the first child within a blossom
    pub first_blossom_child: [Option<CompactNodeIndex>; N],
    /// the number of defect nodes reported by the dual module, not necessarily all the defect nodes
    pub count_defects: usize,
    /// the number of allocated blossoms
    pub count_blossoms: usize,
}

/// the primal node is designed to have exactly 8 fields (32 bytes or 8 bytes in total, w/wo u16_index feature).
/// this simplifies the design on
#[cfg_attr(any(test, feature = "std"), derive(Debug))]
#[derive(Clone)]
pub struct PrimalNode {
    /// an active outer blossom can have three different grow states, but the state of an inner node
    /// (those created as the children of another blossom) is None
    pub grow_state: Option<CompactGrowState>,
    /// the parent in the alternating tree, or `NODE_NONE` if it doesn't have a parent;
    /// when the node is a root node, sibling means the parent in the alternating tree;
    /// when the node is within a blossom, then the parent means the parent blossom
    pub parent: Option<CompactNodeIndex>,
    /// the starting of the children, whether the children in the blossom cycle or in an alternating tree
    pub first_child: Option<CompactNodeIndex>,
    /// the index of one remaining sibling if there exists any, otherwise `NODE_NONE`;
    /// when the node is a root node, sibling means some + node that has the same parent in the alternating tree, or
    ///     it means the temporary match;
    /// when the node is within a blossom, sibling means the next node in the odd cycle
    pub sibling: Option<CompactNodeIndex>,
    /// a link between another node. Depending on the state of a node, the link has different meanings:
    /// when the node is a root node, then the link is pointing to its parent;
    /// when the node is within a blossom, then the link is pointing to its sibling (the odd cycle)
    pub link: TouchingLink,
}

impl<const N: usize, const DOUBLE_N: usize> PrimalNodes<N, DOUBLE_N> {
    pub fn new() -> Self {
        debug_assert_eq!(N * 2, DOUBLE_N);
        const PRIMAL_NODE_NONE: Option<PrimalNode> = None;
        Self {
            buffer: [PRIMAL_NODE_NONE; DOUBLE_N],
            first_blossom_child: [None; N],
            count_defects: 0,
            count_blossoms: 0,
        }
    }

    pub fn clear(&mut self) {
        self.count_defects = 0;
        self.count_blossoms = 0;
    }

    fn prepare_defects_up_to(&mut self, defect_index: CompactNodeIndex) {
        if defect_index.get() as usize >= self.count_defects {
            for index in self.count_defects..=defect_index.get() as usize {
                self.buffer[index as usize] = None;
            }
            self.count_defects = defect_index.get() as usize + 1;
        }
    }

    /// make sure the defect node is set up correctly, especially because the primal module
    /// doesn't really know how many defects are there. In fact, the software primal module
    /// may not eventually holding all the defects because of the offloading, i.e., pre-decoding.
    /// This function is supposed to be called multiple times, whenever a node index is reported to the primal.
    pub fn check_node_index(&mut self, node_index: CompactNodeIndex) {
        if self.is_blossom(node_index) {
            debug_assert!(
                (node_index.get() as usize - N) < self.count_blossoms,
                "blossoms should always be created by the primal module"
            );
        } else {
            self.prepare_defects_up_to(node_index);
            if self.buffer[node_index.get() as usize].is_none() {
                self.buffer[node_index.get() as usize] = Some(PrimalNode::new());
            }
        }
    }

    pub fn is_blossom(&self, node_index: CompactNodeIndex) -> bool {
        debug_assert!(
            (node_index.get() as usize) < DOUBLE_N,
            "node index too large, leading to overflow"
        );
        if node_index.get() < N as CompactNodeNum {
            false
        } else {
            debug_assert!(
                (node_index.get() as usize - N) < self.count_blossoms,
                "blossoms should always be created by the primal module"
            );
            true
        }
    }

    pub fn has_node(&self, node_index: CompactNodeIndex) -> bool {
        get!(self.buffer, node_index.get() as usize).is_some()
    }

    pub fn get_node(&self, node_index: CompactNodeIndex) -> &PrimalNode {
        usu!(get!(self.buffer, node_index.get() as usize).as_ref())
    }

    pub fn get_node_mut(&mut self, node_index: CompactNodeIndex) -> &mut PrimalNode {
        usu!(get_mut!(self.buffer, node_index.get() as usize).as_mut())
    }

    pub fn get_first_blossom_child(&self, blossom_index: CompactNodeIndex) -> CompactNodeIndex {
        debug_assert!(self.is_blossom(blossom_index) && self.has_node(blossom_index));
        usu!(self.first_blossom_child[blossom_index.get() as usize - N])
    }

    pub fn get_defect(&self, defect_index: CompactNodeIndex) -> &PrimalNode {
        debug_assert!(!self.is_blossom(defect_index));
        debug_assert!(
            (defect_index.get() as usize) < self.count_defects,
            "cannot get an uninitialized defect node"
        );
        self.get_node(defect_index)
    }

    pub fn get_blossom(&self, blossom_index: CompactNodeIndex) -> &PrimalNode {
        debug_assert!(self.is_blossom(blossom_index));
        self.get_node(blossom_index)
    }

    pub fn iterate_blossom_children(&self, blossom_index: CompactNodeIndex, mut func: impl FnMut(CompactNodeIndex)) {
        let mut child_index = Some(self.get_first_blossom_child(blossom_index));
        while child_index.is_some() {
            func(usu!(child_index));
            child_index = self.get_node(usu!(child_index)).sibling;
        }
    }

    pub fn iterate_blossom_children_with_touching(
        &self,
        blossom_index: CompactNodeIndex,
        mut func: impl FnMut(CompactNodeIndex, ((CompactNodeIndex, CompactVertexIndex), (CompactNodeIndex, CompactVertexIndex))),
    ) {
        let mut child_index = Some(self.get_first_blossom_child(blossom_index));
        while child_index.is_some() {
            let node = self.get_node(usu!(child_index));
            let link = &node.link;
            func(
                usu!(child_index),
                (
                    (usu!(link.touch), usu!(link.through)),
                    (usu!(link.peer_touch), usu!(link.peer_through)),
                ),
            );
            child_index = node.sibling;
        }
    }

    pub fn iterate_perfect_matching(&self, mut func: impl FnMut(CompactNodeIndex, CompactMatchTarget, &TouchingLink)) {
        // report from small index to large index
        unimplemented!()
    }

    /// get the outer blossom of this possibly inner node
    pub fn get_outer_blossom(&self, mut node_index: CompactNodeIndex) -> CompactNodeIndex {
        loop {
            let node = self.get_node(node_index);
            if node.grow_state.is_none() {
                debug_assert!(node.parent.is_some(), "an inner node must have a outer parent blossom");
                node_index = usu!(node.parent);
            } else {
                return node_index;
            }
        }
    }

    pub fn get_grow_state(&self, node_index: CompactNodeIndex) -> CompactGrowState {
        debug_assert!(
            self.get_node(node_index).grow_state.is_some(),
            "cannot get grow state of an inner node"
        );
        usu!(self.get_node(node_index).grow_state)
    }

    pub fn set_grow_state(
        &mut self,
        node_index: CompactNodeIndex,
        grow_state: CompactGrowState,
        dual_module: &mut impl DualInterface,
    ) {
        debug_assert!(self.get_node(node_index).is_outer_blossom(), "cannot set inner node");
        self.get_node_mut(node_index).grow_state = Some(grow_state);
        dual_module.set_grow_state(node_index, grow_state);
    }

    pub fn temporary_match(
        &mut self,
        dual_module: &mut impl DualInterface,
        node_1: CompactNodeIndex,
        node_2: CompactNodeIndex,
        touch_1: CompactNodeIndex,
        touch_2: CompactNodeIndex,
        vertex_1: CompactVertexIndex,
        vertex_2: CompactVertexIndex,
    ) {
        self.set_grow_state(node_1, CompactGrowState::Stay, dual_module);
        self.set_grow_state(node_2, CompactGrowState::Stay, dual_module);
        let primal_node_1 = self.get_node_mut(node_1);
        primal_node_1.remove_from_alternating_tree();
        primal_node_1.sibling = Some(node_2);
        primal_node_1.link.touch = Some(touch_1);
        primal_node_1.link.through = Some(vertex_1);
        primal_node_1.link.peer_touch = Some(touch_2);
        primal_node_1.link.peer_through = Some(vertex_2);
        let primal_node_2 = self.get_node_mut(node_2);
        primal_node_2.remove_from_alternating_tree();
        primal_node_2.sibling = Some(node_2);
        primal_node_2.link.touch = Some(touch_2);
        primal_node_2.link.through = Some(vertex_2);
        primal_node_2.link.peer_touch = Some(touch_1);
        primal_node_2.link.peer_through = Some(vertex_1);
    }

    pub fn temporary_match_with_link(
        &mut self,
        dual_module: &mut impl DualInterface,
        node_1: CompactNodeIndex,
        link_1: &TouchingLink,
        node_2: CompactNodeIndex,
    ) {
        self.temporary_match(
            dual_module,
            node_1,
            node_2,
            usu!(link_1.touch),
            usu!(link_1.peer_touch),
            usu!(link_1.through),
            usu!(link_1.peer_through),
        );
    }

    pub fn temporary_match_virtual_vertex(
        &mut self,
        dual_module: &mut impl DualInterface,
        node: CompactNodeIndex,
        touch: CompactNodeIndex,
        vertex: CompactVertexIndex,
        virtual_vertex: CompactVertexIndex,
    ) {
        self.set_grow_state(node, CompactGrowState::Stay, dual_module);
        let primal_node = self.get_node_mut(node);
        primal_node.remove_from_alternating_tree();
        primal_node.sibling = None;
        primal_node.link.touch = Some(touch);
        primal_node.link.through = Some(vertex);
        primal_node.link.peer_touch = None;
        primal_node.link.peer_through = Some(virtual_vertex);
    }

    /// allocate a blank blossom
    pub fn allocate_blossom(&mut self, first_blossom_child: CompactNodeIndex) -> CompactNodeIndex {
        debug_assert!(self.count_blossoms < N, "blossom overflow");
        self.buffer[N + self.count_blossoms] = Some(PrimalNode::new());
        self.first_blossom_child[self.count_blossoms] = Some(first_blossom_child);
        let blossom_index = ni!(N + self.count_blossoms);
        self.count_blossoms += 1;
        blossom_index
    }

    /// dispose a blossom, after expanding it
    pub fn dispose_blossom(&mut self, blossom_index: CompactNodeIndex) {
        debug_assert!(self.is_blossom(blossom_index), "do not dispose a defect vertex");
        debug_assert!(self.has_node(blossom_index), "do not dispose twice");
        self.buffer[blossom_index.get() as usize].as_mut().take();
        self.first_blossom_child[blossom_index.get() as usize - N].as_mut().take();
    }
}

impl PrimalNode {
    pub fn new() -> Self {
        Self {
            grow_state: Some(CompactGrowState::Grow),
            parent: None,
            first_child: None,
            sibling: None,
            link: TouchingLink::new(),
        }
    }

    pub fn is_outer_blossom(&self) -> bool {
        self.grow_state.is_some()
    }

    /// check if the node is not matched or not in any alternating tree
    pub fn is_free(&self) -> bool {
        debug_assert!(self.is_outer_blossom(), "should not ask whether an inner node is free");
        !self.in_alternating_tree() && self.link.touch.is_none()
    }

    pub fn is_matched(&self) -> bool {
        // here we use link.touch to judge whether its matched, to distinguish two cases:
        // 1. match to another node (sibling = Some) 2. match to virtual vertex (sibling = None)
        debug_assert!(self.is_outer_blossom(), "should not ask whether an inner node is matched");
        !self.in_alternating_tree() && self.link.touch.is_some()
    }

    pub fn in_alternating_tree(&self) -> bool {
        debug_assert!(
            self.is_outer_blossom(),
            "should not ask whether an inner node is in an alternating tree"
        );
        self.parent.is_some() || self.first_child.is_some()
    }

    pub fn remove_from_alternating_tree(&mut self) {
        debug_assert!(
            self.is_outer_blossom(),
            "should not remove an inner node from alternating tree"
        );
        self.parent = None;
        self.first_child = None;
    }

    pub fn remove_from_matching(&mut self) {
        debug_assert!(self.is_outer_blossom(), "should not remove an inner node from matching");
        debug_assert!(self.is_matched());
        self.sibling = None;
        self.link.touch = None;
        self.link.through = None;
        self.link.peer_touch = None;
        self.link.peer_through = None;
    }
}

impl PrimalNode {
    pub fn get_matched(&self) -> CompactMatchTarget {
        debug_assert!(self.is_matched());
        if let Some(node_index) = self.sibling {
            CompactMatchTarget::Peer(node_index)
        } else {
            CompactMatchTarget::VirtualVertex(usu!(self.link.peer_through))
        }
    }
}

impl TouchingLink {
    pub fn new() -> Self {
        Self {
            touch: None,
            through: None,
            peer_touch: None,
            peer_through: None,
        }
    }

    pub fn is_none(&self) -> bool {
        self.touch.is_none() && self.through.is_none() && self.peer_touch.is_none() && self.peer_through.is_none()
    }
}

#[cfg(any(test, feature = "std"))]
impl<const N: usize, const DOUBLE_N: usize> std::fmt::Debug for PrimalNodes<N, DOUBLE_N> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("Nodes")
            .field(
                "defects",
                &(0..self.count_defects as usize)
                    .map(|index| &self.buffer[index])
                    .collect::<std::vec::Vec<_>>(),
            )
            .field(
                "blossoms",
                &(0..self.count_blossoms as usize)
                    .map(|index| (&self.buffer[N + index], self.first_blossom_child[index]))
                    .collect::<std::vec::Vec<_>>(),
            )
            .finish()
    }
}
