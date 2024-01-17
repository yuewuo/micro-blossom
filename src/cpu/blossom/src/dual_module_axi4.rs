//! Dual Module implemented in Scala (SpinalHDL) with AXI4 interface and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph.
//! It simulates the complete MicroBlossom module, which provides a AXI4 memory-mapped interface.
//!

use crate::dual_module_adaptor::*;
use crate::resources::*;
use crate::util::*;
use embedded_blossom::extern_c::MicroBlossomHardwareInfo;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use rand::{distributions::Alphanumeric, Rng};
use scan_fmt::*;
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

pub struct DualModuleAxi4Driver {
    pub link: Mutex<Link>,
    pub host_name: String,
    pub use64bus: bool,
    pub context_id: u16,
    pub simulation_duration: Duration,
    pub with_waveform: bool,
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
        with_waveform: bool,
    ) -> std::io::Result<Self> {
        // TODO: later on support offloading
        micro_blossom.offloading.0.clear();

        let use64bus = true;
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
        write!(writer, "{}\n", if with_waveform { "with waveform" } else { "no waveform" })?;
        write!(writer, "{}\n", if use64bus { "64 bits bus" } else { "32 bits bus" })?;
        line.clear();
        reader.read_line(&mut line)?;
        assert_eq!(line, "simulation started\n");
        let mut value = Self {
            host_name,
            use64bus,
            context_id: 0,
            with_waveform,
            simulation_duration: Duration::ZERO,
            link: Mutex::new(Link {
                port,
                child,
                reader,
                writer,
            }),
        };
        value.reset();
        Ok(value)
    }

    pub fn new_with_name(initializer: &SolverInitializer, host_name: String) -> std::io::Result<Self> {
        // in simulation, positions doesn't matter because it's not going to affect the timing constraint
        Self::new_with_name_raw(MicroBlossomSingle::new_initializer_only(initializer), host_name, cfg!(test))
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

    pub fn execute_instruction(&mut self, instruction: Instruction32, context_id: u16) {
        if self.use64bus {
            let data = (instruction.0 as u64) | ((context_id as u64) << 32);
            self.memory_write_64(4096, data).unwrap();
        } else {
            unimplemented!()
        }
    }

    pub fn get_hardware_info(&mut self) -> MicroBlossomHardwareInfo {
        MicroBlossomHardwareInfo {
            version: self.memory_read_32(8).unwrap(),
            context_depth: self.memory_read_32(12).unwrap(),
            obstacle_channels: self.memory_read_8(16).unwrap(),
        }
    }
}

impl DualStacklessDriver for DualModuleAxi4Driver {
    fn reset(&mut self) {
        self.execute_instruction(Instruction32::reset(), self.context_id)
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.execute_instruction(Instruction32::set_speed(node, speed), self.context_id)
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        // write!(self.link.lock().unwrap().writer, "set_blossom({node}, {blossom})\n").unwrap();
        unimplemented!()
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        // write!(self.link.lock().unwrap().writer, "find_obstacle()\n").unwrap();
        // let mut line = String::new();
        // self.link.lock().unwrap().reader.read_line(&mut line).unwrap();
        // // println!("find_obstacle -> {}", line);
        // if line.starts_with("NonZeroGrow(") {
        //     let (length, grown) = scan_fmt!(&line, "NonZeroGrow({d}), {d}", Weight, Weight).unwrap();
        //     (
        //         if length == i32::MAX as Weight {
        //             CompactObstacle::None
        //         } else {
        //             CompactObstacle::GrowLength {
        //                 length: length as CompactWeight,
        //             }
        //         },
        //         grown as CompactWeight,
        //     )
        // } else if line.starts_with("Conflict(") {
        //     let (node_1, node_2, touch_1, touch_2, vertex_1, vertex_2, grown) = scan_fmt!(
        //         &line,
        //         "Conflict({d}, {d}, {d}, {d}, {d}, {d}), {d}",
        //         NodeIndex,
        //         NodeIndex,
        //         NodeIndex,
        //         NodeIndex,
        //         NodeIndex,
        //         NodeIndex,
        //         Weight
        //     )
        //     .unwrap();
        //     (
        //         CompactObstacle::Conflict {
        //             node_1: ni!(node_1).option(),
        //             node_2: if node_2 == i32::MAX as NodeIndex {
        //                 None.into()
        //             } else {
        //                 ni!(node_2).option()
        //             },
        //             touch_1: ni!(touch_1).option(),
        //             touch_2: if touch_2 == i32::MAX as NodeIndex {
        //                 None.into()
        //             } else {
        //                 ni!(touch_2).option()
        //             },
        //             vertex_1: ni!(vertex_1),
        //             vertex_2: ni!(vertex_2),
        //         },
        //         grown as CompactWeight,
        //     )
        // } else {
        //     unreachable!()
        // }
        unimplemented!()
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        // write!(self.link.lock().unwrap().writer, "add_defect({vertex}, {node})\n").unwrap();
        unimplemented!()
    }
}

impl DualTrackedDriver for DualModuleAxi4Driver {
    fn set_maximum_growth(&mut self, length: CompactWeight) {
        // write!(self.link.lock().unwrap().writer, "set_maximum_growth({length})\n").unwrap();
        unimplemented!()
    }
}

impl FusionVisualizer for DualModuleAxi4Driver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // write!(self.link.lock().unwrap().writer, "snapshot({abbrev})\n").unwrap();
        // let mut line = String::new();
        // self.link.lock().unwrap().reader.read_line(&mut line).unwrap();
        // std::thread::sleep(std::time::Duration::from_millis(1000));
        // serde_json::from_str(&line).unwrap()
        unimplemented!()
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
        if self.with_waveform {
            // only delete binary but keep original waveforms
            match std::fs::remove_dir_all(format!("../../../simWorkspace/MicroBlossomHost/{}/rtl", self.host_name)) {
                Err(e) => println!("Could not remove rtl folder: {}", e),
                Ok(_) => println!("Successfully remove rtl folder"),
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
