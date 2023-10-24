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
    pub count_grow: usize,
}

impl DualStacklessDriver for DualModuleCounterDriver {
    fn clear(&mut self) {
        self.count_set_speed = 0;
        self.count_set_blossom = 0;
        self.count_grow = 0;
    }
    fn set_speed(&mut self, node: CompactNodeIndex, speed: CompactGrowState) {
        self.count_set_speed += 1;
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.count_set_blossom += 1;
    }
    fn find_obstacle(&mut self) -> MaxUpdateLength {
        unimplemented!()
    }
    fn grow(&mut self, _length: CompactWeight) {
        self.count_grow += 1;
    }
}

impl DualModuleCounterDriver {
    pub fn new() -> Self {
        Self {
            count_set_speed: 0,
            count_set_blossom: 0,
            count_grow: 0,
        }
    }
}
