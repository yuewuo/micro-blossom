//! Interface between primal and dual module
//!
//! Provides the primal module with necessary interface to control the dual module, including
//! setting the speed of node, creating/expanding a blossom, etc.
//!
//! Provides the dual module with necessary information about the blossom structure, etc.
//!

use crate::util::*;

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
}
