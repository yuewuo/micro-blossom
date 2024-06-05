use crate::binding::extern_c::*;
use cty::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;

pub struct DualDriver {
    pub context_id: uint16_t,
}

impl DualDriver {
    pub const fn new() -> Self {
        Self { context_id: 0 }
    }
}

impl DualStacklessDriver for DualDriver {
    fn reset(&mut self) {
        unsafe { execute_instruction(Instruction32::reset().0, self.context_id) };
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        unsafe { execute_instruction(Instruction32::set_speed(node, speed).0, self.context_id) };
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        unsafe { execute_instruction(Instruction32::set_blossom(node, blossom).0, self.context_id) };
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        unsafe { get_single_readout(self.context_id) }.into_obstacle()
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        unsafe { execute_instruction(Instruction32::add_defect_vertex(vertex, node).0, self.context_id) };
    }
}

impl DualTrackedDriver for DualDriver {
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        unsafe { set_maximum_growth(maximum_growth as u16, self.context_id) };
        self.find_obstacle()
    }
}
