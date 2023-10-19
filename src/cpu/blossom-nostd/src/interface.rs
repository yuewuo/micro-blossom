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
    NonZeroGrow {
        /// when length is null,
        length: Weight,
    },
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
    BlossomNeedExpand {
        blossom: NodeIndex,
    },
}

pub trait PrimalInterface {
    /// reset the primal module
    fn clear(&mut self);

    /// query if a node is a blossom node
    fn is_blossom(&self, node_index: NodeIndex) -> bool;

    /// query the structure of a blossom
    fn iterate_blossom_children(&self, blossom_index: NodeIndex, func: impl FnMut(&Self, NodeIndex));
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

    /// grow a specific length
    fn grow(&mut self, length: Weight);
}
