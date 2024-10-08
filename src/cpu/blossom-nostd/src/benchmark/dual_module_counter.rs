//! Dual Module Counter
//!
//! Only counts the number of operations.
//!

use crate::dual_module_stackless::*;
use crate::interface::*;
use crate::util::*;

pub type DualModuleCounter = DualModuleStackless<DualModuleCounterDriver>;

pub struct DualModuleCounterDriver {
    pub count_set_speed: usize,
    pub count_set_blossom: usize,
}

impl DualStacklessDriver for DualModuleCounterDriver {
    fn reset(&mut self) {
        self.count_set_speed = 0;
        self.count_set_blossom = 0;
    }
    fn set_speed(&mut self, _is_blossom: bool, _node: CompactNodeIndex, _speed: CompactGrowState) {
        self.count_set_speed += 1;
    }
    fn set_blossom(&mut self, _node: CompactNodeIndex, _blossom: CompactNodeIndex) {
        self.count_set_blossom += 1;
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        unimplemented_or_loop!()
    }
    fn add_defect(&mut self, _vertex: CompactVertexIndex, _node: CompactNodeIndex) {
        unimplemented_or_loop!()
    }
}

impl DualModuleCounterDriver {
    pub const fn new() -> Self {
        Self {
            count_set_speed: 0,
            count_set_blossom: 0,
        }
    }
}
