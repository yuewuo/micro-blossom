//! Primal Module Embedded
//!
//! An embedded implementation of the primal module that assumes a maximum number of nodes;
//! Also, thanks to primal offloading, a primal module doesn't have to remember all the defect nodes.
//! It only initializes a defect node when it's encountered.
//! Besides, the primal module specifies the index of the blossom, which usually starts from an address
//! that is guaranteed to be distinguishable from defect vertices.
//! Only in this way, we can safely use primal offloading without worrying about mixing with a created blossom.
//!

use crate::util::*;
use heapless::Vec;

#[cfg_attr(any(test, feature = "std"), derive(Debug))]
pub struct PrimalModuleEmbedded<const N: usize, const DOUBLE_N: usize> {
    /// the alternating tree nodes
    pub nodes: Nodes<N, DOUBLE_N>,
}

pub struct Nodes<const N: usize, const DOUBLE_N: usize> {
    /// defect nodes starting from 0, blossom nodes starting from DOUBLE_N/2
    pub buffer: Vec<Node, DOUBLE_N>,
    /// the number of defect nodes reported by the dual module, not necessarily all the defect nodes
    pub count_defects: NodeNum,
    /// the number of allocated blossoms
    pub count_blossoms: NodeNum,
}

#[derive(Clone)]
pub struct Node {
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

impl<const N: usize, const DOUBLE_N: usize> PrimalModuleEmbedded<N, DOUBLE_N> {
    pub fn new() -> Self {
        Self { nodes: Nodes::new() }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }
}

impl<const N: usize, const DOUBLE_N: usize> Nodes<N, DOUBLE_N> {
    pub fn new() -> Self {
        debug_assert_eq!(N * 2, DOUBLE_N);
        let mut buffer = Vec::new();
        buffer.resize(DOUBLE_N, Node::none()).unwrap();
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
    /// This function is supposed to be called multiple times, whenever a defect vertex is reported to the primal.
    pub fn check_defect(&mut self, defect_index: NodeIndex) {
        debug_assert!(
            (defect_index as usize) < N,
            "defect index too large, overlapping with blossom"
        );
        self.prepare_defects_up_to(defect_index);
        if self.buffer[defect_index as usize].is_none() {
            self.buffer[defect_index as usize].create_some(defect_index);
        }
    }

    /// This function is supposed to be called multiple times, whenever a blossom node is reported to the primal.
    /// Blossoms are created by the primal module, so they should be within the proper index range.
    pub fn check_blossom(&mut self, blossom_index: NodeIndex) {
        debug_assert!(
            (blossom_index as usize) >= N,
            "blossom index too small, overlapping with defect nodes"
        );
        debug_assert!(
            (blossom_index as usize) < DOUBLE_N,
            "blossom index too large, leading to overflow"
        );
        let local_index = blossom_index - N as NodeIndex;
        assert!(
            local_index < self.count_blossoms,
            "blossoms should always be created by the primal module"
        );
    }
}

impl Node {
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
impl<const N: usize, const DOUBLE_N: usize> std::fmt::Debug for Nodes<N, DOUBLE_N> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_struct("Nodes")
            .field("defects", &SlicedVec::new(&self.buffer, 0, self.count_defects as usize))
            .field("blossoms", &SlicedVec::new(&self.buffer, N, N + self.count_blossoms as usize))
            .finish()
    }
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for Node {
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
    }

    #[test]
    fn primal_module_debug_print() {
        // cargo test primal_module_debug_print -- --nocapture
        const N: usize = 100;
        const DOUBLE_N: usize = 2 * N;
        let mut primal_module: PrimalModuleEmbedded<N, DOUBLE_N> = PrimalModuleEmbedded::new();
        println!("{primal_module:?}");
        primal_module.nodes.check_defect(3);
        primal_module.nodes.check_defect(1);
        primal_module.nodes.check_defect(4);
        println!("{primal_module:?}");
    }
}
