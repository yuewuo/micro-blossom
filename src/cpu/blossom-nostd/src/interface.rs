//! Interface between primal and dual module
//!
//! Provides the primal module with necessary interface to control the dual module, including
//! setting the speed of node, creating/expanding a blossom, etc.
//!
//! Provides the dual module with necessary information about the blossom structure, etc.
//!

use crate::util::*;

#[derive(Debug)]
pub enum CompactObstacle {
    /// nothing to do
    None,
    /// a non-negative growth
    GrowLength {
        /// when length is 0, one just need to call grow(0) for erasure errors to propagate
        length: CompactWeight,
    },
    /// some conflict needs the primal module to resolve
    Conflict {
        /// node_1 is assumed to be always normal node
        node_1: CompactNodeIndex,
        /// node_2 could be NODE_NONE, which means it's touching a virtual vertex `vertex_2`
        node_2: Option<CompactNodeIndex>,
        touch_1: CompactNodeIndex,
        touch_2: Option<CompactNodeIndex>,
        vertex_1: CompactVertexIndex,
        vertex_2: CompactVertexIndex,
    },
    /// a blossom needs to be expanded
    BlossomNeedExpand { blossom: CompactNodeIndex },
}

pub trait PrimalInterface {
    /// reset the primal module
    fn reset(&mut self);

    /// query if a node is a blossom node
    fn is_blossom(&self, node_index: CompactNodeIndex) -> bool;

    /// query the detailed structure of a blossom including the data of the touching information;
    /// the format is (node, ((touch, through), (peer_touch, peer_through))), (peer, ......;
    /// this is different
    fn iterate_blossom_children(
        &self,
        blossom_index: CompactNodeIndex,
        func: impl FnMut(&Self, CompactNodeIndex, &TouchingLink),
    );

    /// resolve one obstacle, returning whether the obstacle is handled properly;
    /// this design allows multiple level of primal module to be designed, each handling a simple subset
    fn resolve(&mut self, dual_module: &mut impl DualInterface, max_update_length: CompactObstacle) -> bool;

    /// iterate the perfect matching between defect nodes
    fn iterate_perfect_matching(&mut self, func: impl FnMut(&Self, CompactNodeIndex, CompactMatchTarget, &TouchingLink));

    /// if the node is matched with a specific virtual index; note that the node index might be outdated
    /// , so it is necessary to check for the latest node index. return true if it's broken otherwise false
    fn break_with_virtual_vertex(
        &mut self,
        dual_module: &mut impl DualInterface,
        virtual_vertex: CompactVertexIndex,
        hint_node_index: CompactNodeIndex,
    ) -> bool;
}

pub trait DualInterface {
    /// reset the dual module
    fn reset(&mut self);

    /// create a blossom
    fn create_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: CompactNodeIndex);

    /// expand a blossom
    fn expand_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: CompactNodeIndex);

    /// set the speed of a node
    fn set_speed(&mut self, is_blossom: bool, node_index: CompactNodeIndex, grow_state: CompactGrowState);

    /// find an obstacle and return the amount of growth from last return
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight);

    /// add a defect at given vertex
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex);
}

impl CompactObstacle {
    pub fn is_obstacle(&self) -> bool {
        !(matches!(self, Self::None) || matches!(self, Self::GrowLength { .. }))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn is_finite_growth(&self) -> bool {
        matches!(self, Self::GrowLength { .. })
    }
}
