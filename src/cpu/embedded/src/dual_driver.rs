use crate::binding::extern_c::*;
use crate::util::*;
use cty::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;

pub struct DualDriver<const MAX_CONFLICT_CHANNELS: usize = 8> {
    pub context_id: uint16_t,
    pub conflicts_store: ConflictsStore<MAX_CONFLICT_CHANNELS>,
}

impl<const MAX_CC: usize> DualDriver<MAX_CC> {
    pub const fn new(conflict_channels: u8, context_id: uint16_t) -> Self {
        Self {
            context_id,
            conflicts_store: ConflictsStore::new(conflict_channels),
        }
    }
}

impl<const CC: usize> DualStacklessDriver for DualDriver<CC> {
    fn reset(&mut self) {
        unsafe { execute_instruction(Instruction32::reset().0, self.context_id) };
        unsafe { set_maximum_growth(0, self.context_id) };
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        unsafe { execute_instruction(Instruction32::set_speed(node, speed).0, self.context_id) };
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        unsafe { execute_instruction(Instruction32::set_blossom(node, blossom).0, self.context_id) };
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        // first check whether there are some unhandled conflicts in the store
        if let Some(conflict) = self.conflicts_store.pop() {
            return (conflict.get_obstacle(), 0);
        }
        // then query the hardware
        unsafe { self.conflicts_store.get_conflicts(self.context_id) };
        // check again
        let grown = self.conflicts_store.head.accumulated_grown as CompactWeight;
        let growable = self.conflicts_store.head.growable;
        if growable == u16::MAX {
            (CompactObstacle::None, grown)
        } else if growable != 0 {
            (
                CompactObstacle::GrowLength {
                    length: growable as CompactWeight,
                },
                grown,
            )
        } else {
            // find a single obstacle from the list of obstacles
            if let Some(conflict) = self.conflicts_store.pop() {
                return (conflict.get_obstacle(), grown);
            }
            // when this happens, the DualDriverTracked should check for BlossomNeedExpand event
            // this is usually triggered by reaching maximum growth set by the DualDriverTracked
            (CompactObstacle::GrowLength { length: 0 }, grown)
        }
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        unsafe { execute_instruction(Instruction32::add_defect_vertex(vertex, node).0, self.context_id) };
    }
}

impl<const CC: usize> DualTrackedDriver for DualDriver<CC> {
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        unsafe { set_maximum_growth(maximum_growth as u16, self.context_id) };
        let result = self.find_obstacle();
        unsafe { set_maximum_growth(0, self.context_id) }; // clear maximum growth to avoid any spontaneous growth
        result
    }
}
