//! Primal Nodes
//!
//! A data type that keeps track of (partial) defect nodes and all blossom nodes.
//! The defect nodes are kept partial because of the pre-decoder that never reports some of the
//! defects if they are not evolves in potential complex matchings.
//!

use crate::util::*;
use heapless::Vec;

pub struct PrimalNodes<const N: usize, const DOUBLE_N: usize> {
    /// defect nodes starting from 0, blossom nodes starting from DOUBLE_N/2
    pub buffer: Vec<PrimalNode, DOUBLE_N>,
    /// the number of defect nodes reported by the dual module, not necessarily all the defect nodes
    pub count_defects: NodeNum,
    /// the number of allocated blossoms
    pub count_blossoms: NodeNum,
}

#[derive(Clone)]
pub struct PrimalNode {
    /// the root of an alternating tree, or `NODE_NONE` if this node is not initialized
    pub root: NodeIndex,
    /// the parent in the alternating tree, or `NODE_NONE` if it doesn't have a parent
    pub parent: Link,
    /// the starting of the children
    pub first_child: NodeIndex,
    /// the index of one remaining sibling if there exists any, otherwise `NODE_NONE`;
    pub sibling: NodeIndex,
    /// the depth in the alternating tree, root has 0 depth
    pub depth: NodeNum,
    /// temporary match with another node, (target, touching_grandson)
    pub matching: Link,
}

#[derive(Clone)]
pub struct Link {
    /// the index of the peer
    pub peer: NodeIndex,
    /// touching through node index
    pub touching: NodeIndex,
}

impl<const N: usize, const DOUBLE_N: usize> PrimalNodes<N, DOUBLE_N> {
    pub fn new() -> Self {
        debug_assert_eq!(N * 2, DOUBLE_N);
        let mut buffer = Vec::new();
        buffer.resize(DOUBLE_N, PrimalNode::none()).unwrap();
        Self {
            buffer,
            count_defects: 0,
            count_blossoms: 0,
        }
    }

    pub fn clear(&mut self) {
        self.count_defects = 0;
        self.count_blossoms = 0;
    }

    fn prepare_defects_up_to(&mut self, defect_index: NodeIndex) {
        if defect_index >= self.count_defects {
            for index in self.count_defects..=defect_index {
                self.buffer[index as usize].set_none();
            }
            self.count_defects = defect_index + 1;
        }
    }

    /// make sure the defect node is set up correctly, especially because the primal module
    /// doesn't really know how many defects are there. In fact, the software primal module
    /// may not eventually holding all the defects because of the offloading, i.e., pre-decoding.
    /// This function is supposed to be called multiple times, whenever a node index is reported to the primal.
    pub fn check_node_index(&mut self, node_index: NodeIndex) {
        if self.is_blossom(node_index) {
            assert!(
                (node_index - N as NodeIndex) < self.count_blossoms,
                "blossoms should always be created by the primal module"
            );
        } else {
            self.prepare_defects_up_to(node_index);
            if self.buffer[node_index as usize].is_none() {
                self.buffer[node_index as usize].create_some(node_index);
            }
        }
    }

    pub fn is_blossom(&self, node_index: NodeIndex) -> bool {
        debug_assert!((node_index as usize) < DOUBLE_N, "node index too large, leading to overflow");
        if node_index < N as NodeIndex {
            false
        } else {
            debug_assert!(
                (node_index - N as NodeIndex) < self.count_blossoms,
                "blossoms should always be created by the primal module"
            );
            true
        }
    }

    pub fn get_node(&self, node_index: NodeIndex) -> &PrimalNode {
        &self.buffer[node_index as usize]
    }

    pub fn get_defect(&self, defect_index: NodeIndex) -> &PrimalNode {
        debug_assert!(!self.is_blossom(defect_index));
        debug_assert!(defect_index < self.count_defects, "cannot get an uninitialized defect node");
        self.get_node(defect_index)
    }

    pub fn get_blossom(&self, blossom_index: NodeIndex) -> &PrimalNode {
        debug_assert!(self.is_blossom(blossom_index));
        self.get_node(blossom_index)
    }

    pub fn iterate_blossom_children(&self, blossom_index: NodeIndex, mut func: impl FnMut(NodeIndex)) {
        let blossom = self.get_blossom(blossom_index);
        let mut child_index = blossom.first_child;
        while child_index != NODE_NONE {
            func(child_index);
            child_index = self.get_node(child_index).sibling;
        }
    }
}

impl PrimalNode {
    pub fn none() -> Self {
        Self {
            root: NODE_NONE, // mark as uninitialized node
            parent: Link::none(),
            first_child: NODE_NONE,
            sibling: NODE_NONE,
            depth: 0,
            matching: Link::none(),
        }
    }

    /// since most of the defect nodes will never be accessed by the primal module when offloading,
    /// we optimize speed by simply marking them as "None"
    fn set_none(&mut self) {
        self.root = NODE_NONE;
    }

    fn is_none(&self) -> bool {
        self.root == NODE_NONE
    }

    fn create_some(&mut self, node_index: NodeIndex) {
        self.root = node_index; // set the root as itself
        self.parent = Link::none();
        self.first_child = NODE_NONE;
        self.sibling = NODE_NONE;
        self.depth = 0; // root has 0 depth
        self.matching = Link::none();
    }
}

impl Link {
    pub fn none() -> Self {
        Self {
            peer: NODE_NONE,
            touching: NODE_NONE,
        }
    }

    fn is_none(&self) -> bool {
        self.peer == NODE_NONE
    }
}

#[cfg(any(test, feature = "std"))]
impl<const N: usize, const DOUBLE_N: usize> std::fmt::Debug for PrimalNodes<N, DOUBLE_N> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("Nodes")
            .field("defects", &SlicedVec::new(&self.buffer, 0, self.count_defects as usize))
            .field("blossoms", &SlicedVec::new(&self.buffer, N, N + self.count_blossoms as usize))
            .finish()
    }
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for PrimalNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_none() {
            formatter.write_str("None")
        } else {
            formatter
                .debug_struct("Node")
                .field("parent", &self.parent)
                .field("first_child", &self.first_child)
                .field("sibling", &self.sibling)
                .field("depth", &self.depth)
                .field("matching", &self.matching)
                .finish()
        }
    }
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for Link {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_none() {
            formatter.write_str("None")
        } else {
            formatter
                .debug_struct("Link")
                .field("peer", &self.peer)
                .field("touching", &self.touching)
                .finish()
        }
    }
}
