//! Primal Nodes
//!
//! A data type that keeps track of (partial) defect nodes and all blossom nodes.
//! The defect nodes are kept partial because of the pre-decoder that never reports some of the
//! defects if they are not evolves in potential complex matchings.
//!

use crate::util::*;

pub struct PrimalNodes<const N: usize, const DOUBLE_N: usize> {
    /// defect nodes starting from 0, blossom nodes starting from DOUBLE_N/2
    pub buffer: [Option<PrimalNode>; DOUBLE_N],
    /// the number of defect nodes reported by the dual module, not necessarily all the defect nodes
    pub count_defects: NodeNum,
    /// the number of allocated blossoms
    pub count_blossoms: NodeNum,
}

/// the primal node is designed to have exactly 8 fields (32 bytes or 8 bytes in total, w/wo u16_index feature).
/// this simplifies the design on
#[derive(Clone)]
pub struct PrimalNode {
    /// the parent in the alternating tree, or `NODE_NONE` if it doesn't have a parent;
    /// when the node is a root node, sibling means the parent in the alternating tree;
    /// when the node is within a blossom, then the parent means the parent blossom
    pub parent: NodeIndex,
    /// the starting of the children, whether the children in the blossom cycle or in an alternating tree
    pub first_child: NodeIndex,
    /// the index of one remaining sibling if there exists any, otherwise `NODE_NONE`;
    /// when the node is a root node, sibling means some + node that has the same parent in the alternating tree;
    /// when the node is a blossom node, sibling means the next
    pub sibling: NodeIndex,
    /// a link between another node. Depending on the state of a node, the link has different meanings:
    /// when the node is a root node, then the link is pointing to its parent;
    /// when the node is within a blossom, then the link is pointing to its sibling (the circle)
    pub link: Option<Link>,
}

#[derive(Clone)]
pub struct Link {
    /// touching through node index
    pub touch: NodeIndex,
    /// touching through vertex
    pub through: VertexIndex,
    /// peer touches myself through node index
    pub peer_touch: NodeIndex,
    /// peer touches myself through vertex
    pub peer_through: VertexIndex,
}

impl<const N: usize, const DOUBLE_N: usize> PrimalNodes<N, DOUBLE_N> {
    pub fn new() -> Self {
        debug_assert_eq!(N * 2, DOUBLE_N);
        const PRIMAL_NODE_NONE: Option<PrimalNode> = None;
        Self {
            buffer: [PRIMAL_NODE_NONE; DOUBLE_N],
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
                self.buffer[index as usize] = None;
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
                self.buffer[node_index as usize] = Some(PrimalNode::new());
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
        self.buffer[node_index as usize].as_ref().unwrap()
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

    pub fn iterate_blossom_children_with_touching(
        &self,
        blossom_index: NodeIndex,
        mut func: impl FnMut(NodeIndex, ((NodeIndex, VertexIndex), (NodeIndex, VertexIndex))),
    ) {
        let blossom = self.get_blossom(blossom_index);
        let mut child_index = blossom.first_child;
        while child_index != NODE_NONE {
            let node = self.get_node(child_index);
            let link = node.link.as_ref().unwrap();
            func(
                child_index,
                ((link.touch, link.through), (link.peer_touch, link.peer_through)),
            );
            child_index = node.sibling;
        }
    }
}

impl PrimalNode {
    pub fn new() -> Self {
        Self {
            parent: NODE_NONE,
            first_child: NODE_NONE,
            sibling: NODE_NONE,
            link: None,
        }
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
                &(N..N + self.count_blossoms as usize)
                    .map(|index| &self.buffer[index])
                    .collect::<std::vec::Vec<_>>(),
            )
            .finish()
    }
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for PrimalNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("Node")
            .field("parent", &self.parent)
            .field("first_child", &self.first_child)
            .field("sibling", &self.sibling)
            .field("link", &self.link)
            .finish()
    }
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for Link {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("Link")
            .field("touch", &self.touch)
            .field("through", &self.through)
            .field("peer_touch", &self.peer_touch)
            .field("peer_through", &self.peer_through)
            .finish()
    }
}
