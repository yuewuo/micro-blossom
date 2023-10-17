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
#[cfg_attr(any(test, feature = "std"), derive(Debug))]
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
    pub temporary_match: Link,
}

#[derive(Clone)]
#[cfg_attr(any(test, feature = "std"), derive(Debug))]
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

impl<const N: usize, const DOUBLE_N: usize> Nodes<N, DOUBLE_N> {
    pub fn new() -> Self {
        debug_assert_eq!(N * 2, DOUBLE_N);
        let mut buffer = Vec::new();
        buffer.resize(DOUBLE_N, Node::new()).unwrap();
        Self {
            buffer,
            count_defects: 0,
            count_blossoms: 0,
        }
    }
}

impl Node {
    pub fn new() -> Self {
        Self {
            root: NODE_NONE, // mark as uninitialized node
            parent: Link::new(),
            first_child: NODE_NONE,
            sibling: NODE_NONE,
            depth: 0,
            temporary_match: Link::new(),
        }
    }
}

impl Link {
    pub fn new() -> Self {
        Self {
            peer: NODE_NONE,
            touching: NODE_NONE,
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
        let primal_module: PrimalModuleEmbedded<N, DOUBLE_N> = PrimalModuleEmbedded::new();
        println!("{primal_module:?}");
    }
}
