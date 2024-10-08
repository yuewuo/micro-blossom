//! Interface between primal and dual module
//!
//! Provides the primal module with necessary interface to control the dual module, including
//! setting the speed of node, creating/expanding a blossom, etc.
//!
//! Provides the dual module with necessary information about the blossom structure, etc.
//!

use crate::util::*;
#[cfg(feature = "serde")]
use serde::*;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
        node_1: OptionCompactNodeIndex,
        /// node_2 could be NODE_NONE, which means it's touching a virtual vertex `vertex_2`
        node_2: OptionCompactNodeIndex,
        touch_1: OptionCompactNodeIndex,
        touch_2: OptionCompactNodeIndex,
        vertex_1: CompactVertexIndex,
        vertex_2: CompactVertexIndex,
    },
    /// a blossom needs to be expanded
    BlossomNeedExpand { blossom: CompactNodeIndex },
}

impl CompactObstacle {
    pub fn reduce(resp1: CompactObstacle, resp2: CompactObstacle) -> CompactObstacle {
        if matches!(resp1, CompactObstacle::None) {
            return resp2;
        }
        if matches!(resp2, CompactObstacle::None) {
            return resp1;
        }
        if !matches!(resp1, CompactObstacle::GrowLength { .. }) {
            return resp1;
        }
        if !matches!(resp2, CompactObstacle::GrowLength { .. }) {
            return resp2;
        }
        let CompactObstacle::GrowLength { length: length1 } = resp1 else {
            unreachable!()
        };
        let CompactObstacle::GrowLength { length: length2 } = resp2 else {
            unreachable!()
        };
        CompactObstacle::GrowLength {
            length: core::cmp::min(length1, length2),
        }
    }

    pub fn fix_conflict_order(&mut self) {
        if let Self::Conflict {
            node_1,
            node_2,
            touch_1,
            touch_2,
            vertex_1,
            vertex_2,
        } = self
        {
            if node_1.is_none() {
                debug_assert!(node_2.is_some(), "at least one of node_1 and node_2 should be some");
                *self = Self::Conflict {
                    node_1: *node_2,
                    node_2: *node_1,
                    touch_1: *touch_2,
                    touch_2: *touch_1,
                    vertex_1: *vertex_2,
                    vertex_2: *vertex_1,
                }
            }
        }
    }
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
