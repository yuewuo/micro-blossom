//! Interface between primal and dual module
//!
//! Provides the primal module with necessary interface to control the dual module, including
//! setting the speed of node, creating/expanding a blossom, etc.
//!
//! Provides the dual module with necessary information about the blossom structure, etc.
//!

use crate::util::*;

#[derive(Debug)]
pub enum MaxUpdateLength {
    /// nothing to do
    None,
    /// a non-negative growth
    GrowLength {
        /// when length is 0, one just need to call grow(0) for erasure errors to propagate
        length: Weight,
    },
    /// some conflict needs the primal module to resolve
    Conflict {
        /// node_1 is assumed to be always normal node
        node_1: NodeIndex,
        /// node_2 could be NODE_NONE, which means it's touching a virtual vertex `vertex_2`
        node_2: NodeIndex,
        touch_1: NodeIndex,
        touch_2: NodeIndex,
        vertex_1: VertexIndex,
        vertex_2: VertexIndex,
    },
    /// a blossom needs to be expanded
    BlossomNeedExpand { blossom: NodeIndex },
}

pub trait PrimalInterface {
    /// reset the primal module
    fn clear(&mut self);

    /// query if a node is a blossom node
    fn is_blossom(&self, node_index: NodeIndex) -> bool;

    /// query the structure of a blossom
    fn iterate_blossom_children(&self, blossom_index: NodeIndex, func: impl FnMut(&Self, NodeIndex));

    /// query the detailed structure of a blossom including the data of the touching information
    fn iterate_blossom_children_with_touching(
        &self,
        blossom_index: NodeIndex,
        func: impl FnMut(&Self, NodeIndex, ((NodeIndex, VertexIndex), (NodeIndex, VertexIndex))),
    );

    /// resolve one obstacle
    fn resolve(&mut self, dual_module: &mut impl DualInterface, max_update_length: MaxUpdateLength);

    /// return the perfect matching between nodes
    fn iterate_perfect_matching(&mut self, func: impl FnMut(&Self, NodeIndex));
}

pub trait DualInterface {
    /// reset the dual module
    fn clear(&mut self);

    /// create a blossom
    fn create_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: NodeIndex);

    /// expand a blossom
    fn expand_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: NodeIndex);

    /// set the speed of a node
    fn set_grow_state(&mut self, node_index: NodeIndex, grow_state: GrowState);

    /// compute the maximum length to update, or to find an obstacle
    fn compute_maximum_update_length(&mut self) -> MaxUpdateLength;

    /// grow a specific length; however in an offloaded system, this should never be called from software
    fn grow(&mut self, length: Weight);
}

impl MaxUpdateLength {
    pub fn is_obstacle(&self) -> bool {
        !(matches!(self, Self::None) || matches!(self, Self::GrowLength { .. }))
    }
}
