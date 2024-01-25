//! Dual Module implemented in Scala (SpinalHDL) with AXI4 interface and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph.
//! It simulates the complete MicroBlossom module, which provides a AXI4 memory-mapped interface.
//!

use crate::dual_module_adaptor::*;
use crate::resources::*;
use crate::util::*;
use derivative::Derivative;
use embedded_blossom::extern_c::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use rand::{distributions::Alphanumeric, Rng};
use scan_fmt::*;
use serde::Serialize;
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

#[derive(Serialize, Derivative)]
#[derivative(Default)]
pub struct DualConfig {
    #[derivative(Default(value = "*dual_config_default::WITH_WAVEFORM"))]
    pub with_waveform: bool,
    #[derivative(Default(value = "(*dual_config_default::BUS_TYPE).clone()"))]
    pub bus_type: String,
    #[derivative(Default(value = "*dual_config_default::USE_64_BUS"))]
    pub use_64_bus: bool,
    #[derivative(Default(value = "dual_config_default::env_usize(\"CONTEXT_DEPTH\", 1)"))]
    pub context_depth: usize,
    #[derivative(Default(value = "dual_config_default::env_usize(\"BROADCAST_DELAY\", 1)"))]
    pub broadcast_delay: usize,
    #[derivative(Default(value = "dual_config_default::env_usize(\"CONVERGECAST_DELAY\", 1)"))]
    pub convergecast_delay: usize,
    #[derivative(Default(value = "dual_config_default::env_usize(\"CONFLICT_CHANNELS\", 1)"))]
    pub conflict_channels: usize,
    #[derivative(Default(value = "*dual_config_default::SUPPORT_ADD_DEFECT_VERTEX"))]
    pub support_add_defect_vertex: bool,
    #[derivative(Default(value = "dual_config_default::INJECT_REGISTERS.clone()"))]
    pub inject_registers: Vec<String>,
    #[derivative(Default(value = "dual_config_default::env_usize(\"CLOCK_DIVIDED_BY\", 1)"))]
    pub clock_divide_by: usize,
}

pub const MAX_CONFLICT_CHANNELS: usize = 62;

pub struct DualModuleAxi4Driver {
    pub link: Mutex<Link>,
    pub host_name: String,
    pub context_id: u16,
    pub maximum_growth: Vec<u16>,
    pub simulation_duration: Duration,
    pub dual_config: DualConfig,
    pub head: ReadoutHead,
    pub conflicts: Vec<ReadoutConflict>,
}

pub struct Link {
    pub port: u16,
    pub child: Child,
    pub reader: BufReader<TcpStream>,
    pub writer: LineWriter<TcpStream>,
}

pub type DualModuleAxi4 = DualModuleStackless<DualDriverTracked<DualModuleAxi4Driver, MAX_NODE_NUM>>;

impl DualInterfaceWithInitializer for DualModuleAxi4 {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self {
        DualModuleStackless::new(DualDriverTracked::new(DualModuleAxi4Driver::new(initializer).unwrap()))
    }
}

impl DualModuleAxi4Driver {
    pub fn new_with_name_raw(
        mut micro_blossom: MicroBlossomSingle,
        host_name: String,
        dual_config: DualConfig,
    ) -> std::io::Result<Self> {
        // TODO: later on support offloading
        micro_blossom.offloading.0.clear();
        let hostname = "127.0.0.1";
        let listener = TcpListener::bind(format!("{hostname}:0"))?;
        let port = listener.local_addr()?.port();
        // start the scala simulator host
        println!("Starting Scala simulator host... this may take a while (listening on {hostname}:{port})");
        let child = Command::new("sbt")
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"))
            .arg(format!("runMain microblossom.MicroBlossomHost {hostname} {port} {host_name}"))
            .spawn()?;
        let (socket, _addr) = listener.accept()?;
        let mut reader = BufReader::new(socket.try_clone()?);
        let mut writer = LineWriter::new(socket);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        assert_eq!(line, "MicroBlossomHost v0.0.1, ask for decoding graph\n", "handshake error");
        write!(writer, "{}\n", serde_json::to_string(&micro_blossom).unwrap())?;
        dual_config.write_to(&mut writer)?;
        line.clear();
        reader.read_line(&mut line)?;
        assert_eq!(line, "simulation started\n");
        assert!(dual_config.conflict_channels <= MAX_CONFLICT_CHANNELS);
        let conflict_channels = dual_config.conflict_channels;
        let mut value = Self {
            host_name,
            context_id: 0,
            maximum_growth: vec![0; dual_config.context_depth],
            dual_config,
            simulation_duration: Duration::ZERO,
            link: Mutex::new(Link {
                port,
                child,
                reader,
                writer,
            }),
            head: ReadoutHead::new(),
            conflicts: (0..conflict_channels).map(|_| ReadoutConflict::invalid()).collect(),
        };
        value.reset();
        Ok(value)
    }

    pub fn new_with_name(initializer: &SolverInitializer, host_name: String) -> std::io::Result<Self> {
        // in simulation, positions doesn't matter because it's not going to affect the timing constraint
        Self::new_with_name_raw(
            MicroBlossomSingle::new_initializer_only(initializer),
            host_name,
            Default::default(),
        )
    }

    pub fn new(initializer: &SolverInitializer) -> std::io::Result<Self> {
        let host_name = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        Self::new_with_name(initializer, host_name)
    }

    fn memory_write(&mut self, num_bytes: usize, address: usize, data: usize) -> std::io::Result<()> {
        let begin = Instant::now();
        write!(self.link.lock().unwrap().writer, "write({num_bytes}, {address}, {data})\n")?;
        self.simulation_duration += begin.elapsed();
        Ok(())
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
        let begin = Instant::now();
        let mut link = self.link.lock().unwrap();
        write!(link.writer, "read({num_bytes}, {address})\n")?;
        let mut line = String::new();
        link.reader.read_line(&mut line)?;
        let value = scan_fmt!(&line, "{d}", usize).unwrap();
        self.simulation_duration += begin.elapsed();
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
        if self.dual_config.use_64_bus {
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
        self.head.maximum_growth = raw_head as u16;
        self.head.accumulated_grown = (raw_head >> 16) as u16;
        self.head.growable = (raw_head >> 32) as u16;
        for i in 0..self.conflicts.len() {
            let conflict_base = base + 32 + i * 16;
            let raw_1 = self.memory_read_64(conflict_base)?;
            let raw_2 = self.memory_read_64(conflict_base + 8)?;
            let conflict = &mut self.conflicts[i];
            conflict.node_1 = raw_1 as u16;
            conflict.node_2 = (raw_1 >> 16) as u16;
            conflict.touch_1 = (raw_1 >> 32) as u16;
            conflict.touch_2 = (raw_1 >> 48) as u16;
            conflict.vertex_1 = raw_2 as u16;
            conflict.vertex_2 = (raw_2 >> 16) as u16;
            conflict.valid = (raw_2 >> 32) as u8;
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
        // after user writing
        self.get_conflicts(self.context_id).unwrap();
        let grown = self.head.accumulated_grown as CompactWeight;
        if self.head.growable == u16::MAX {
            (CompactObstacle::None, grown)
        } else if self.head.growable != 0 {
            (
                CompactObstacle::GrowLength {
                    length: self.head.growable as CompactWeight,
                },
                grown,
            )
        } else {
            // find a single obstacle from the list of obstacles
            for conflict in self.conflicts.iter() {
                if conflict.valid != 0 {
                    return (
                        CompactObstacle::Conflict {
                            node_1: ni!(conflict.node_1).option(),
                            node_2: if conflict.node_2 == u16::MAX {
                                None.into()
                            } else {
                                ni!(conflict.node_2).option()
                            },
                            touch_1: ni!(conflict.touch_1).option(),
                            touch_2: if conflict.touch_2 == u16::MAX {
                                None.into()
                            } else {
                                ni!(conflict.touch_2).option()
                            },
                            vertex_1: ni!(conflict.vertex_1),
                            vertex_2: ni!(conflict.vertex_2),
                        },
                        grown,
                    );
                }
            }
            unreachable!()
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
        assert_eq!(self.dual_config.context_depth, 1, "context snapshot is not yet supported");
        write!(self.link.lock().unwrap().writer, "snapshot({abbrev})\n").unwrap();
        let mut line = String::new();
        self.link.lock().unwrap().reader.read_line(&mut line).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1000));
        serde_json::from_str(&line).unwrap()
    }
}

// https://stackoverflow.com/questions/30538004/how-do-i-ensure-that-a-spawned-child-process-is-killed-if-my-app-panics
impl Drop for DualModuleAxi4Driver {
    fn drop(&mut self) {
        let need_to_kill: bool = (|| {
            if write!(self.link.lock().unwrap().writer, "quit\n").is_ok() {
                let wait_time = std::time::Duration::from_millis(1000);
                if let Ok(Some(status)) = self.link.lock().unwrap().child.wait_timeout(wait_time) {
                    return !status.success();
                }
            }
            true
        })();
        if need_to_kill {
            match self.link.lock().unwrap().child.kill() {
                Err(e) => println!("Could not kill Scala process: {}", e),
                Ok(_) => println!("Successfully killed Scala process"),
            }
        } else {
            println!("Scala process quit normally");
        }
        if self.dual_config.with_waveform {
            // only delete binary but keep original waveforms
            if !dual_config_default::is_set("KEEP_RTL_FOLDER") {
                match std::fs::remove_dir_all(format!("../../../simWorkspace/MicroBlossomHost/{}/rtl", self.host_name)) {
                    Err(e) => println!("Could not remove rtl folder: {}", e),
                    Ok(_) => println!("Successfully remove rtl folder"),
                }
            }
            match std::fs::remove_dir_all(format!("../../../simWorkspace/MicroBlossomHost/{}/verilator", self.host_name)) {
                Err(e) => println!("Could not remove verilator folder: {}", e),
                Ok(_) => println!("Successfully remove verilator folder"),
            }
        } else {
            match std::fs::remove_dir_all(format!("../../../simWorkspace/MicroBlossomHost/{}", self.host_name)) {
                Err(e) => println!("Could not remove build folder: {}", e),
                Ok(_) => println!("Successfully remove build folder"),
            }
        }
    }
}

pub mod dual_config_default {
    use lazy_static::lazy_static;
    use std::env;
    pub fn is_set(name: &str) -> bool {
        match env::var(name) {
            Ok(value) => value != "",
            Err(_) => false,
        }
    }
    pub fn env_usize(name: &str, default: usize) -> usize {
        match env::var(name) {
            Ok(value) => value.parse().unwrap(),
            Err(_) => default,
        }
    }
    lazy_static! {
        pub static ref WITH_WAVEFORM: bool = (cfg!(test) || is_set("WITH_WAVEFORM")) && !is_set("NO_WAVEFORM");
        pub static ref BUS_TYPE: String = env::var("BUS_TYPE").unwrap_or("AxiLite4".to_string());
        pub static ref USE_64_BUS: bool = !is_set("USE_32_BUS");
        pub static ref SUPPORT_ADD_DEFECT_VERTEX: bool = !is_set("NO_ADD_DEFECT_VERTEX");
        pub static ref INJECT_REGISTERS: Vec<String> = match env::var("INJECT_REGISTERS") {
            Ok(value) => value.split(',').map(|a| a.to_string()).collect(),
            Err(_) => vec![],
        };
    }
}
impl DualConfig {
    pub fn write_to(&self, writer: &mut impl Write) -> std::io::Result<()> {
        let value = serde_json::to_value(self).unwrap();
        let object = value.as_object().unwrap();
        for (key, value) in object {
            write!(writer, "{} = {}\n", key, value)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_module_rtl::tests::*;
    use crate::mwpm_solver::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_axi4_basic_1() {
        // cargo test dual_module_axi4_basic_1 -- --nocapture
        let visualize_filename = "dual_module_axi4_basic_1.json".to_string();
        let defect_vertices = vec![0, 4, 8];
        dual_module_axi4_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    }

    #[test]
    fn dual_module_axi4_basic_2() {
        // cargo test dual_module_axi4_basic_2 -- --nocapture
        let visualize_filename = "dual_module_axi4_basic_2.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    #[test]
    fn dual_module_axi4_basic_3() {
        // cargo test dual_module_axi4_basic_3 -- --nocapture
        let visualize_filename = "dual_module_axi4_basic_3.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// debug infinite loop
    /// reason: the write stage logic is implemented wrongly: only when the overall speed is positive
    ///   should it report an obstacle; otherwise just report whatever the maxGrowth value is
    #[test]
    fn dual_module_axi4_debug_1() {
        // cargo test dual_module_axi4_debug_1 -- --nocapture
        let visualize_filename = "dual_module_axi4_debug_1.json".to_string();
        let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
        dual_module_axi4_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    #[test]
    fn dual_module_axi4_debug_compare_1() {
        // cargo test dual_module_axi4_debug_compare_1 -- --nocapture
        let visualize_filename = "dual_module_axi4_debug_compare_1.json".to_string();
        let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
        dual_module_rtl_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    pub fn dual_module_axi4_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> SolverDualAxi4 {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer| {
                SolverDualAxi4::new_with_name(initializer, visualize_filename.as_str().trim_end_matches(".json").to_string())
                //.with_max_iterations(30)  // this is helpful when debugging infinite loops
            },
        )
    }
}
