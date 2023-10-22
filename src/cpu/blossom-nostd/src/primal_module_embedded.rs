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
                if let Some(node_2) = node_2 {
                    assert!(node_1 != node_2, "one cannot conflict with itself");
                }
                cfg_if::cfg_if! {
                    if #[cfg(feature="obstacle_potentially_outdated")] {
                        if self.nodes.is_blossom(node_1) && !self.nodes.has_node(node_1) {
                            return; // outdated event
                        }
                        // also convert the conflict to between the outer blossom
                        node_1 = self.nodes.get_blossom_root(node_1);
                        if let Some(some_node_2) = node_2 {
                            if self.nodes.is_blossom(some_node_2) && !self.nodes.has_node(some_node_2) {
                                return; // outdated event
                            }
                            node_2 = Some(self.nodes.get_blossom_root(some_node_2));
                        }
                    }
                }
                self.nodes.check_node_index(node_1);
                self.nodes.check_node_index(touch_1);
                if let Some(node_2) = node_2 {
                    self.nodes.check_node_index(node_2);
                    self.nodes.check_node_index(touch_2.unwrap());
                    self.resolve_conflict(dual_module, node_1, node_2, touch_1, touch_2.unwrap(), vertex_1, vertex_2);
                } else {
                    self.resolve_conflict_virtual(dual_module, node_1, touch_1, vertex_1, vertex_2);
                }
            }
            MaxUpdateLength::BlossomNeedExpand { blossom } => {
                self.resolve_blossom_need_expand(dual_module, blossom);
            }
            _ => unimplemented!(),
        }
    }

    /// return the perfect matching between nodes
    fn iterate_perfect_matching(&mut self, func: impl FnMut(&Self, CompactNodeIndex)) {
        unimplemented!()
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
        // println!("primal_node_1: {primal_node_1:?}, primal_node_2: {primal_node_2:?}");
        unimplemented!()
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
        unimplemented!()
    }

    /// handle an up-to-date blossom need expand event
    pub fn resolve_blossom_need_expand(&mut self, dual_module: &mut impl DualInterface, blossom: CompactNodeIndex) {
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
