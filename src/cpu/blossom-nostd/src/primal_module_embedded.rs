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

    fn is_blossom(&self, node_index: NodeIndex) -> bool {
        self.nodes.is_blossom(node_index)
    }

    /// query the structure of a blossom
    fn iterate_blossom_children(&self, blossom_index: NodeIndex, mut func: impl FnMut(&Self, NodeIndex)) {
        self.nodes
            .iterate_blossom_children(blossom_index, |node_index| func(self, node_index));
    }

    /// query the structure of a blossom with detailed information of touching points
    fn iterate_blossom_children_with_touching(
        &self,
        blossom_index: NodeIndex,
        mut func: impl FnMut(&Self, NodeIndex, ((NodeIndex, VertexIndex), (NodeIndex, VertexIndex))),
    ) {
        self.nodes
            .iterate_blossom_children_with_touching(blossom_index, |node_index, touching_info| {
                func(self, node_index, touching_info)
            });
    }

    /// resolve one obstacle
    fn resolve(&mut self, dual_module: &mut impl DualInterface, obstacle: MaxUpdateLength) {
        debug_assert!(obstacle.is_obstacle());
        match obstacle {
            MaxUpdateLength::Conflict { .. } => {
                unimplemented!()
            }
            MaxUpdateLength::BlossomNeedExpand { blossom } => {
                unimplemented!()
            }
            _ => unimplemented!(),
        }
    }

    /// return the perfect matching between nodes
    fn iterate_perfect_matching(&mut self, func: impl FnMut(&Self, NodeIndex)) {
        unimplemented!()
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
        primal_module.nodes.check_node_index(3);
        primal_module.nodes.check_node_index(1);
        primal_module.nodes.check_node_index(4);
        println!("{primal_module:?}");
    }
}
