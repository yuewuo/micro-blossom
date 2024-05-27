//! Dual Module implemented in Scala (SpinalHDL) with AXI4 interface and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph.
//! It simulates the complete MicroBlossom module, which provides a AXI4 memory-mapped interface.
//!

use crate::mwpm_solver::*;
use crate::resources::*;
use crate::simulation_tcp_client::*;
use crate::util::*;
use embedded_blossom::extern_c::*;
use embedded_blossom::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use scan_fmt::*;
use serde::*;

pub struct DualModuleAxi4Driver {
    pub client: SimulationTcpClient,
    pub context_id: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualAxi4Config {
    #[serde(default = "Default::default")]
    pub sim_config: SimulationConfig,
    #[serde(default = "random_name_16")]
    pub name: String,
}

pub type DualModuleAxi4 = DualModuleStackless<DualDriverTracked<DualModuleAxi4Driver, MAX_NODE_NUM>>;

impl SolverTrackedDual for DualModuleAxi4Driver {
    fn new_from_graph_config(graph: MicroBlossomSingle, config: serde_json::Value) -> Self {
        Self::new(graph, serde_json::from_value(config).unwrap()).unwrap()
    }
    fn fuse_layer(&mut self, layer_id: usize) {
        self.execute_instruction(Instruction32::load_syndrome_external(ni!(layer_id)), self.context_id)
            .unwrap();
    }
    fn get_pre_matchings(&self, belonging: DualModuleInterfaceWeak) -> PerfectMatching {
        self.client.get_pre_matchings(belonging)
    }
}

impl DualModuleAxi4Driver {
    pub fn new(micro_blossom: MicroBlossomSingle, config: DualAxi4Config) -> std::io::Result<Self> {
        let mut value = Self {
            client: SimulationTcpClient::new("MicroBlossomHost", micro_blossom, config.name, config.sim_config)?,
            context_id: 0,
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
            assert!(context_id < 1024);
            self.memory_write_32(8192 + 4 * context_id as usize, instruction.0)
        }
    }

    pub fn get_hardware_info(&mut self) -> std::io::Result<MicroBlossomHardwareInfo> {
        let raw_1 = self.memory_read_64(8)?;
        let raw_2 = self.memory_read_64(16)?;
        Ok(MicroBlossomHardwareInfo {
            version: raw_1 as u32,
            context_depth: (raw_1 >> 32) as u32,
            conflict_channels: raw_2 as u8,
            vertex_bits: (raw_2 >> 8) as u8,
            weight_bits: (raw_2 >> 16) as u8,
            instruction_buffer_depth: (raw_2 >> 24) as u8,
        })
    }

    pub const READOUT_BASE: usize = 128 * 1024;

    /// this function issues a FindObstacle instruction; to let the accelerator calculate the results
    /// while the CPU doing other things. If the data is prefetched, then the read will be very fast;
    /// if not pre-fetched, or if new instructions are written to this context, then reading the conflicts
    /// will automatically issue a FindObstacle instruction inside the hardware
    pub fn pre_fetch_conflicts(&mut self, context_id: u16) -> std::io::Result<()> {
        self.execute_instruction(Instruction32::find_obstacle(), context_id)
    }

    pub fn get_conflicts(&mut self, context_id: u16) -> std::io::Result<SingleReadout> {
        let base_address = Self::READOUT_BASE + 128 * context_id as usize;
        let readout_address = base_address + 32;
        // self.pre_fetch_conflicts(context_id)?; // optional
        let readout = unsafe {
            let mut readout_union = SingleReadoutUnion { raw: [0, 0] };
            readout_union.raw[0] = self.memory_read_64(readout_address)?;
            readout_union.raw[1] = self.memory_read_64(readout_address + 8)?;
            readout_union.readout
        };
        self.memory_write_16(base_address, 0)?;
        Ok(readout)
    }

    pub fn set_maximum_growth(&mut self, maximum_growth: u16, context_id: u16) -> std::io::Result<()> {
        let base = Self::READOUT_BASE + 128 * context_id as usize + 16;
        self.memory_write_16(base, maximum_growth)
    }

    pub fn get_maximum_growth(&mut self, context_id: u16) -> std::io::Result<u16> {
        let base = Self::READOUT_BASE + 128 * context_id as usize + 16;
        self.memory_read_16(base)
    }

    pub fn clear_error_counter(&mut self) -> std::io::Result<()> {
        self.memory_write_32(48, 0)
    }
    pub fn get_error_counter(&mut self) -> std::io::Result<u32> {
        self.memory_read_32(48)
    }

    pub fn sanity_check(&mut self) -> std::io::Result<()> {
        let error_counter = self.get_error_counter()?;
        if error_counter > 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("error counter = {error_counter}"),
            ));
        }
        Ok(())
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
        let readout = self.get_conflicts(self.context_id).unwrap();
        // check again
        let grown = readout.accumulated_grown as CompactWeight;
        let growable = readout.max_growable;
        if growable == u8::MAX {
            (CompactObstacle::None, grown)
        } else if growable != 0 {
            (
                CompactObstacle::GrowLength {
                    length: growable as CompactWeight,
                },
                grown,
            )
        } else if readout.conflict_valid != 0 {
            let conflict = CompactObstacle::Conflict {
                node_1: ni!(readout.node_1).option(),
                node_2: if readout.node_2 == u16::MAX {
                    None.into()
                } else {
                    ni!(readout.node_2).option()
                },
                touch_1: ni!(readout.touch_1).option(),
                touch_2: if readout.touch_2 == u16::MAX {
                    None.into()
                } else {
                    ni!(readout.touch_2).option()
                },
                vertex_1: ni!(readout.vertex_1),
                vertex_2: ni!(readout.vertex_2),
            };
            (conflict, grown)
        } else {
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
        self.find_obstacle()
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
    use crate::dual_module_adaptor::tests::*;
    use fusion_blossom::example_codes::*;
    use fusion_blossom::util::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_axi4_get_hardware_info() {
        // WITH_WAVEFORM=1 KEEP_RTL_FOLDER=1 cargo test dual_module_axi4_get_hardware_info -- --nocapture
        let code = CodeCapacityPlanarCode::new(3, 0.1, 500);
        let mut driver = DualModuleAxi4Driver::new(
            MicroBlossomSingle::new(&code.get_initializer(), &code.get_positions()),
            serde_json::from_value(json!({ "name": "axi4_get_hardware_info" })).unwrap(),
        )
        .unwrap();
        let hardware_info = driver.get_hardware_info().unwrap();
        println!("{hardware_info:?}");
        assert_eq!(hardware_info.context_depth, 1);
        assert_eq!(hardware_info.conflict_channels, 1);
        assert_eq!(hardware_info.vertex_bits, 7);
        assert_eq!(hardware_info.weight_bits, 10); // maximum weight = 1000 < 1024
        assert_eq!(hardware_info.instruction_buffer_depth, 10);
    }

    fn dual_module_axi4_register_test(graph: MicroBlossomSingle, config: DualAxi4Config) -> DualModuleAxi4Driver {
        let mut driver = DualModuleAxi4Driver::new(graph.clone(), config.clone()).unwrap();
        let hardware_info = driver.get_hardware_info().unwrap();
        println!("hardware_info: {hardware_info:?}");
        assert_eq!(hardware_info.conflict_channels, 1);
        assert_eq!(hardware_info.context_depth as usize, config.sim_config.context_depth);
        driver.sanity_check().unwrap();
        // test maximum growth value set and read
        assert_eq!(driver.get_maximum_growth(0).unwrap(), 0, "the default should be 0");
        for value in [100, 0, 65535, 0, 200, 300, 0] {
            driver.set_maximum_growth(value, 0).unwrap();
            assert_eq!(driver.get_maximum_growth(0).unwrap(), value);
            driver.sanity_check().unwrap();
        }
        // test maximum growth value of different context
        if config.sim_config.context_depth > 1 {
            for value in [100, 0, 65535, 0, 200, 300, 0] {
                driver.set_maximum_growth(value, 1).unwrap();
                assert_eq!(driver.get_maximum_growth(1).unwrap(), value);
                assert_eq!(driver.get_maximum_growth(0).unwrap(), 0, "should not affect context 0");
                driver.sanity_check().unwrap();
            }
        }
        // writing to an out-of-bound context should result in error
        driver.set_maximum_growth(5, config.sim_config.context_depth as u16).unwrap();
        assert_eq!(driver.get_error_counter().unwrap(), 1, "write result in an error");
        driver.clear_error_counter().unwrap();
        driver.get_maximum_growth(config.sim_config.context_depth as u16).unwrap();
        assert_eq!(driver.get_error_counter().unwrap(), 1, "read also result in an error");
        driver.clear_error_counter().unwrap();
        // find any real vertex
        let virtual_vertices: BTreeSet<VertexIndex> = graph.virtual_vertices.iter().cloned().collect();
        let example_vertex = (0..graph.vertex_num).find(|v| !virtual_vertices.contains(v)).unwrap();
        println!("use example vertex {example_vertex}");
        let vertex = ni!(example_vertex);
        let node = ni!(0);
        driver.reset();
        driver.sanity_check().unwrap();
        let (obstacle, grown) = driver.find_obstacle();
        assert_eq!(obstacle, CompactObstacle::None);
        assert_eq!(grown, 0);
        driver.sanity_check().unwrap();
        driver.add_defect(vertex, node);
        driver.sanity_check().unwrap();
        driver
    }

    #[test]
    fn dual_module_axi4_build_test_1() {
        // WITH_WAVEFORM=1 KEEP_RTL_FOLDER=1 cargo test dual_module_axi4_build_test_1 -- --nocapture
        let code = CodeCapacityPlanarCode::new(3, 0.1, 1);
        let mut driver = dual_module_axi4_register_test(
            MicroBlossomSingle::new(&code.get_initializer(), &code.get_positions()),
            serde_json::from_value(json!({ "name": "axi4_build_test_1" })).unwrap(),
        );
        let hardware_info = driver.get_hardware_info().unwrap();
        assert_eq!(hardware_info.vertex_bits, 5);
        assert_eq!(hardware_info.weight_bits, 2);
    }

    #[test]
    fn dual_module_axi4_build_various_configurations() {
        // WITH_WAVEFORM=1 KEEP_RTL_FOLDER=1 cargo test dual_module_axi4_build_various_configurations -- --nocapture
        let sim_configurations = vec![
            json!({}),
            // test variable bus interface
            json!({ "bus_type": "AxiLite4", "use_64_bus": true }), // 64 bit AxiLite4
            json!({ "bus_type": "AxiLite4", "use_64_bus": false }), // 32 bit AxiLite4
            json!({ "bus_type": "Axi4", "use_64_bus": true, "dump_debugger_files": false }), // 64 bit Axi4
        ];
        let code = CodeCapacityPlanarCode::new(3, 0.1, 1);
        for (index, sim_config) in sim_configurations.iter().enumerate() {
            println!("------------------- configuration [{index}]: {sim_config}");
            dual_module_axi4_register_test(
                MicroBlossomSingle::new(&code.get_initializer(), &code.get_positions()),
                serde_json::from_value(json!({ "name": format!("axi4_build_{index}"), "sim_config": sim_config })).unwrap(),
            );
        }
    }

    #[test]
    fn dual_module_axi4_basic_1() {
        // WITH_WAVEFORM=1 KEEP_RTL_FOLDER=1 cargo test dual_module_axi4_basic_1 -- --nocapture
        let visualize_filename = "dual_module_axi4_basic_1.json".to_string();
        let defect_vertices = vec![0, 4, 8];
        dual_module_axi4_basic_standard_syndrome(3, visualize_filename, defect_vertices, json!({}));
    }

    //     #[test]
    //     fn dual_module_axi4_basic_2() {
    //         // cargo test dual_module_axi4_basic_2 -- --nocapture
    //         let visualize_filename = "dual_module_axi4_basic_2.json".to_string();
    //         let defect_vertices = vec![18, 26, 34];
    //         dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    //     }

    //     #[test]
    //     fn dual_module_axi4_basic_3() {
    //         // cargo test dual_module_axi4_basic_3 -- --nocapture
    //         let visualize_filename = "dual_module_axi4_basic_3.json".to_string();
    //         let defect_vertices = vec![16, 26];
    //         dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    //     }

    //     /// debug infinite loop
    //     /// reason: the write stage logic is implemented wrongly: only when the overall speed is positive
    //     ///   should it report an obstacle; otherwise just report whatever the maxGrowth value is
    //     #[test]
    //     fn dual_module_axi4_debug_1() {
    //         // cargo test dual_module_axi4_debug_1 -- --nocapture
    //         let visualize_filename = "dual_module_axi4_debug_1.json".to_string();
    //         let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
    //         dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    //     }

    //     #[test]
    //     fn dual_module_axi4_debug_compare_1() {
    //         // cargo test dual_module_axi4_debug_compare_1 -- --nocapture
    //         let visualize_filename = "dual_module_axi4_debug_compare_1.json".to_string();
    //         let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
    //         dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, false, false);
    //     }

    //     /// debug timing error
    //     /// the primal offloaded grow unit will issue a grow command automatically and retrieve the conflict information
    //     /// however, this is different from
    //     #[test]
    //     fn dual_module_axi4_debug_2() {
    //         // cargo test dual_module_axi4_debug_2 -- --nocapture
    //         let visualize_filename = "dual_module_axi4_debug_2.json".to_string();
    //         let defect_vertices = vec![12, 13, 17, 25, 28, 48, 49];
    //         dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    //     }

    //     #[test]
    //     fn dual_module_axi4_debug_compare_2() {
    //         // cargo test dual_module_axi4_debug_compare_2 -- --nocapture
    //         let visualize_filename = "dual_module_axi4_debug_compare_2.json".to_string();
    //         let defect_vertices = vec![12, 13, 17, 25, 28, 48, 49];
    //         dual_module_scala_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    //     }

    pub fn dual_module_axi4_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
        sim_config: serde_json::Value,
    ) -> SolverEmbeddedAxi4 {
        dual_module_standard_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer, positions| {
                SolverEmbeddedAxi4::new(
                    MicroBlossomSingle::new(initializer, positions),
                    json!({
                        "dual": {
                            "name": visualize_filename.as_str().trim_end_matches(".json").to_string(),
                            "sim_config": sim_config,
                            // "with_max_iterations": 30, // this is helpful when debugging infinite loops
                        }
                    }),
                )
            },
        )
    }
}
