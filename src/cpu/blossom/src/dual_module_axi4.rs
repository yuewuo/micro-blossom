//! Dual Module implemented in Scala (SpinalHDL) with AXI4 interface and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph.
//! It simulates the complete MicroBlossom module, which provides a AXI4 memory-mapped interface.
//!

use crate::dual_module_adaptor::*;
use crate::resources::*;
use crate::simulation_tcp_host::*;
use crate::util::*;
use embedded_blossom::extern_c::*;
use embedded_blossom::util::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use scan_fmt::*;

pub struct DualModuleAxi4Driver {
    pub client: SimulationTcpClient,
    pub context_id: u16,
    pub maximum_growth: Vec<u16>,
    pub conflicts_store: ConflictsStore<MAX_CONFLICT_CHANNELS>,
}

pub type DualModuleAxi4 = DualModuleStackless<DualDriverTracked<DualModuleAxi4Driver, MAX_NODE_NUM>>;

impl DualInterfaceWithInitializer for DualModuleAxi4 {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self {
        let micro_blossom = MicroBlossomSingle::new_initializer_only(initializer);
        let name = random_name_16();
        let sim_config: SimulationConfig = Default::default();
        DualModuleStackless::new(DualDriverTracked::new(
            DualModuleAxi4Driver::new(micro_blossom, name, sim_config).unwrap(),
        ))
    }
}

impl DualModuleAxi4Driver {
    pub fn new(micro_blossom: MicroBlossomSingle, name: String, sim_config: SimulationConfig) -> std::io::Result<Self> {
        let conflict_channels = sim_config.conflict_channels;
        let mut conflicts_store = ConflictsStore::new();
        conflicts_store.reconfigure(conflict_channels as u8);
        let maximum_growth = vec![0; sim_config.context_depth];
        let mut value = Self {
            client: SimulationTcpClient::new("MicroBlossomHost", micro_blossom, name, sim_config)?,
            context_id: 0,
            maximum_growth,
            conflicts_store,
        };
        value.reset();
        Ok(value)
    }

    fn memory_write(&mut self, num_bytes: usize, address: usize, data: usize) -> std::io::Result<()> {
        self.client.write_line(format!("write({num_bytes}, {address}, {data})"))
    }
    pub fn memory_write_64(&mut self, address: usize, data: u64) -> std::io::Result<()> {
        self.memory_write(8, address, data as usize)
    }
    pub fn memory_write_32(&mut self, address: usize, data: u32) -> std::io::Result<()> {
        self.memory_write(4, address, data as usize)
    }
    pub fn memory_write_16(&mut self, address: usize, data: u16) -> std::io::Result<()> {
        self.memory_write(2, address, data as usize)
    }
    pub fn memory_write_8(&mut self, address: usize, data: u8) -> std::io::Result<()> {
        self.memory_write(1, address, data as usize)
    }

    fn memory_read(&mut self, num_bytes: usize, address: usize) -> std::io::Result<usize> {
        let line = self.client.read_line(format!("read({num_bytes}, {address})"))?;
        let value = scan_fmt!(&line, "{d}", usize).unwrap();
        Ok(value)
    }
    pub fn memory_read_64(&mut self, address: usize) -> std::io::Result<u64> {
        self.memory_read(8, address).map(|v| v as u64)
    }
    pub fn memory_read_32(&mut self, address: usize) -> std::io::Result<u32> {
        self.memory_read(4, address).map(|v| v as u32)
    }
    pub fn memory_read_16(&mut self, address: usize) -> std::io::Result<u16> {
        self.memory_read(2, address).map(|v| v as u16)
    }
    pub fn memory_read_8(&mut self, address: usize) -> std::io::Result<u8> {
        self.memory_read(1, address).map(|v| v as u8)
    }

    pub fn execute_instruction(&mut self, instruction: Instruction32, context_id: u16) -> std::io::Result<()> {
        if self.client.sim_config.use_64_bus {
            let data = (instruction.0 as u64) | ((context_id as u64) << 32);
            self.memory_write_64(4096, data)
        } else {
            self.memory_write_32(64 * 1024 + 4 * context_id as usize, instruction.0)
        }
    }

    pub fn get_hardware_info(&mut self) -> std::io::Result<MicroBlossomHardwareInfo> {
        let raw_1 = self.memory_read_64(8)?;
        let raw_2 = self.memory_read_32(16)?;
        Ok(MicroBlossomHardwareInfo {
            version: raw_1 as u32,
            context_depth: (raw_1 >> 32) as u32,
            conflict_channels: raw_2 as u8,
            vertex_bits: (raw_2 >> 8) as u8,
            weight_bits: (raw_2 >> 16) as u8,
            grown_bits: (raw_2 >> 24) as u8,
        })
    }

    pub const READOUT_BASE: usize = 4 * 1024 * 1024;

    pub fn get_conflicts(&mut self, context_id: u16) -> std::io::Result<()> {
        let base = Self::READOUT_BASE + 1024 * context_id as usize;
        self.execute_instruction(Instruction32::find_obstacle(), context_id)?;
        let raw_head = self.memory_read_64(base)?;
        self.conflicts_store.head.maximum_growth = raw_head as u16;
        self.conflicts_store.head.accumulated_grown = (raw_head >> 16) as u16;
        self.conflicts_store.head.growable = (raw_head >> 32) as u16;
        if self.conflicts_store.head.growable == 0 {
            for i in 0..self.conflicts_store.channels as usize {
                let conflict_base = base + 32 + i * 16;
                let raw_1 = self.memory_read_64(conflict_base)?;
                let raw_2 = self.memory_read_64(conflict_base + 8)?;
                let conflict = self.conflicts_store.maybe_uninit_conflict(i);
                conflict.node_1 = raw_1 as u16;
                conflict.node_2 = (raw_1 >> 16) as u16;
                conflict.touch_1 = (raw_1 >> 32) as u16;
                conflict.touch_2 = (raw_1 >> 48) as u16;
                conflict.vertex_1 = raw_2 as u16;
                conflict.vertex_2 = (raw_2 >> 16) as u16;
                conflict.valid = (raw_2 >> 32) as u8;
            }
        }
        Ok(())
    }

    pub fn set_maximum_growth(&mut self, maximum_growth: u16, context_id: u16) -> std::io::Result<()> {
        let base = Self::READOUT_BASE + 1024 * context_id as usize;
        self.memory_write_16(base, maximum_growth)
    }
}

impl DualStacklessDriver for DualModuleAxi4Driver {
    fn reset(&mut self) {
        self.execute_instruction(Instruction32::reset(), self.context_id).unwrap();
        self.set_maximum_growth(0, self.context_id).unwrap();
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.execute_instruction(Instruction32::set_speed(node, speed), self.context_id)
            .unwrap();
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.execute_instruction(Instruction32::set_blossom(node, blossom), self.context_id)
            .unwrap();
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        // first check whether there are some unhandled conflicts in the store
        if let Some(conflict) = self.conflicts_store.pop() {
            return (conflict.get_obstacle(), 0);
        }
        // then query the hardware
        self.get_conflicts(self.context_id).unwrap();
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
        self.execute_instruction(Instruction32::add_defect_vertex(vertex, node), self.context_id)
            .unwrap();
    }
}

impl DualTrackedDriver for DualModuleAxi4Driver {
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        self.set_maximum_growth(maximum_growth as u16, self.context_id).unwrap();
        let result = self.find_obstacle();
        self.set_maximum_growth(0, self.context_id).unwrap(); // clear maximum growth to avoid any spontaneous growth
        result
    }
}

impl FusionVisualizer for DualModuleAxi4Driver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        assert_eq!(
            self.client.sim_config.context_depth, 1,
            "context snapshot is not yet supported"
        );
        let line = self.client.read_line(format!("snapshot({abbrev})")).unwrap();
        serde_json::from_str(&line).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_module_rtl::tests::*;
    use crate::dual_module_scala::tests::*;
    use crate::mwpm_solver::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    // #[test]
    // fn dual_module_axi4_basic_1() {
    //     // cargo test dual_module_axi4_basic_1 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_basic_1.json".to_string();
    //     let defect_vertices = vec![0, 4, 8];
    //     dual_module_axi4_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    // }

    // #[test]
    // fn dual_module_axi4_basic_2() {
    //     // cargo test dual_module_axi4_basic_2 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_basic_2.json".to_string();
    //     let defect_vertices = vec![18, 26, 34];
    //     dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    // }

    // #[test]
    // fn dual_module_axi4_basic_3() {
    //     // cargo test dual_module_axi4_basic_3 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_basic_3.json".to_string();
    //     let defect_vertices = vec![16, 26];
    //     dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    // }

    // /// debug infinite loop
    // /// reason: the write stage logic is implemented wrongly: only when the overall speed is positive
    // ///   should it report an obstacle; otherwise just report whatever the maxGrowth value is
    // #[test]
    // fn dual_module_axi4_debug_1() {
    //     // cargo test dual_module_axi4_debug_1 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_debug_1.json".to_string();
    //     let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
    //     dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    // }

    // #[test]
    // fn dual_module_axi4_debug_compare_1() {
    //     // cargo test dual_module_axi4_debug_compare_1 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_debug_compare_1.json".to_string();
    //     let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
    //     dual_module_rtl_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    // }

    // /// debug timing error
    // /// the primal offloaded grow unit will issue a grow command automatically and retrieve the conflict information
    // /// however, this is different from
    // #[test]
    // fn dual_module_axi4_debug_2() {
    //     // cargo test dual_module_axi4_debug_2 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_debug_2.json".to_string();
    //     let defect_vertices = vec![12, 13, 17, 25, 28, 48, 49];
    //     dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    // }

    // #[test]
    // fn dual_module_axi4_debug_compare_2() {
    //     // cargo test dual_module_axi4_debug_compare_2 -- --nocapture
    //     let visualize_filename = "dual_module_axi4_debug_compare_2.json".to_string();
    //     let defect_vertices = vec![12, 13, 17, 25, 28, 48, 49];
    //     dual_module_scala_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    // }

    pub fn dual_module_axi4_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> Box<SolverDualAxi4> {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer, _| {
                Box::new(
                    SolverDualAxi4::new_with_name(
                        initializer,
                        visualize_filename.as_str().trim_end_matches(".json").to_string(),
                    ), //.with_max_iterations(30)  // this is helpful when debugging infinite loops
                )
            },
        )
    }
}
