use crate::resources::*;
use crate::util::*;
use derivative::Derivative;
use fusion_blossom::dual_module::*;
use fusion_blossom::pointers::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::util::*;
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::process::Child;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use wait_timeout::ChildExt;

pub const MAX_CONFLICT_CHANNELS: usize = 15;

#[derive(Serialize, Deserialize, Derivative, Clone, Debug)]
#[derivative(Default)]
pub struct SimulationConfig {
    #[derivative(Default(value = "simulation_config_default::with_waveform()"))]
    #[serde(default = "simulation_config_default::with_waveform")]
    pub with_waveform: bool,
    #[derivative(Default(value = "simulation_config_default::dump_debugger_files()"))]
    #[serde(default = "simulation_config_default::dump_debugger_files")]
    pub dump_debugger_files: bool,
    #[derivative(Default(value = "simulation_config_default::bus_type()"))]
    #[serde(default = "simulation_config_default::bus_type")]
    pub bus_type: String,
    #[derivative(Default(value = "simulation_config_default::use_64_bus()"))]
    #[serde(default = "simulation_config_default::use_64_bus")]
    pub use_64_bus: bool,
    #[derivative(Default(value = "simulation_config_default::context_depth()"))]
    #[serde(default = "simulation_config_default::context_depth")]
    pub context_depth: usize,
    #[derivative(Default(value = "simulation_config_default::broadcast_delay()"))]
    #[serde(default = "simulation_config_default::broadcast_delay")]
    pub broadcast_delay: usize,
    #[derivative(Default(value = "simulation_config_default::convergecast_delay()"))]
    #[serde(default = "simulation_config_default::convergecast_delay")]
    pub convergecast_delay: usize,
    #[derivative(Default(value = "simulation_config_default::conflict_channels()"))]
    #[serde(default = "simulation_config_default::conflict_channels")]
    pub conflict_channels: usize,
    #[derivative(Default(value = "simulation_config_default::hard_code_weights()"))]
    #[serde(default = "simulation_config_default::hard_code_weights")]
    pub hard_code_weights: bool,
    #[derivative(Default(value = "simulation_config_default::support_add_defect_vertex()"))]
    #[serde(default = "simulation_config_default::support_add_defect_vertex")]
    pub support_add_defect_vertex: bool,
    #[derivative(Default(value = "simulation_config_default::support_offloading()"))]
    #[serde(default = "simulation_config_default::support_offloading")]
    pub support_offloading: bool,
    #[derivative(Default(value = "simulation_config_default::support_layer_fusion()"))]
    #[serde(default = "simulation_config_default::support_layer_fusion")]
    pub support_layer_fusion: bool,
    #[derivative(Default(value = "simulation_config_default::support_load_stall_emulator()"))]
    #[serde(default = "simulation_config_default::support_load_stall_emulator")]
    pub support_load_stall_emulator: bool,
    #[derivative(Default(value = "simulation_config_default::inject_registers()"))]
    #[serde(default = "simulation_config_default::inject_registers")]
    pub inject_registers: Vec<String>,
    #[derivative(Default(value = "simulation_config_default::clock_divide_by()"))]
    #[serde(default = "simulation_config_default::clock_divide_by")]
    pub clock_divide_by: f64,
}

pub struct SimulationTcpClient {
    /// Scala class name, e.g. `DualHost`, `LooperHost`, `MicroBlossomHost`
    pub simulation_name: String,
    /// arbitrary name, used to distinguish between different simulations
    pub name: String,
    link: Mutex<Link>,
    pub compile_wall_time: Duration,
    pub sim_config: SimulationConfig,
}

pub struct Link {
    pub port: u16,
    pub child: Child,
    pub reader: BufReader<TcpStream>,
    pub writer: LineWriter<TcpStream>,
    pub wall_time: Duration,
}

impl SimulationTcpClient {
    pub fn new(
        simulation_name: &str,
        micro_blossom: MicroBlossomSingle,
        name: String,
        sim_config: SimulationConfig,
    ) -> std::io::Result<Self> {
        assert!(sim_config.conflict_channels <= MAX_CONFLICT_CHANNELS);

        let hostname = "127.0.0.1";
        let listener = TcpListener::bind(format!("{hostname}:0"))?;
        let port = listener.local_addr()?.port();
        // start the scala simulator host
        println!("Starting Scala simulator host... this may take a while (listening on {hostname}:{port})");
        let child = SCALA_MICRO_BLOSSOM_RUNNER.run(
            format!("microblossom.{simulation_name}").as_str(),
            [format!("{hostname}"), format!("{port}"), format!("{name}")],
        )?;
        let (socket, _addr) = listener.accept()?;
        let mut reader = BufReader::new(socket.try_clone()?);
        let mut writer = LineWriter::new(socket.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        assert_eq!(
            line,
            format!("{simulation_name} v0.0.1, ask for decoding graph\n"),
            "handshake error"
        );
        write!(writer, "{}\n", serde_json::to_string(&micro_blossom).unwrap())?;
        let compile_wall_time = {
            let simulation_lock = SCALA_SIMULATION_LOCK.lock();
            let compile_begin = Instant::now();
            sim_config.write_to(&mut writer)?;
            line.clear();
            reader.read_line(&mut line)?;
            assert_eq!(line, "simulation started\n");
            drop(simulation_lock);
            compile_begin.elapsed()
        };
        // read operation should not take more than 30s; if failed, it's probably infinite loop in the verilog code
        socket.set_read_timeout(Some(std::time::Duration::from_secs(30))).unwrap();
        Ok(Self {
            simulation_name: simulation_name.to_string(),
            name,
            sim_config,
            compile_wall_time,
            link: Mutex::new(Link {
                port,
                child,
                reader,
                writer,
                wall_time: Duration::ZERO,
            }),
        })
    }

    pub fn write_line(&self, message: String) -> std::io::Result<()> {
        let mut link = self.link.lock().unwrap();
        let begin = Instant::now();
        writeln!(link.writer, "{}", message)?;
        link.wall_time += begin.elapsed();
        Ok(())
    }

    pub fn read_line(&self, message: String) -> std::io::Result<String> {
        let mut link = self.link.lock().unwrap();
        let begin = Instant::now();
        writeln!(link.writer, "{}", message)?;
        let mut line = String::new();
        link.reader.read_line(&mut line)?;
        link.wall_time += begin.elapsed();
        Ok(line)
    }

    pub fn link_wall_time(&self) -> Duration {
        self.link.lock().unwrap().wall_time
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreMatchingData {
    pub edge_index: usize,
    pub node1: u16,
    pub node2: Option<u16>,
    pub touch1: u16,
    pub touch2: Option<u16>,
    pub vertex1: u16,
    pub vertex2: u16,
}

// some helper common functions
impl SimulationTcpClient {
    pub fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        assert_eq!(self.sim_config.context_depth, 1, "context snapshot is not yet supported");
        let line = self.read_line(format!("snapshot({abbrev})")).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    pub fn get_pre_matchings(&self, belonging: DualModuleInterfaceWeak) -> PerfectMatching {
        let line = self.read_line(format!("pre_matchings()")).unwrap();
        let pre_matchings: Vec<PreMatchingData> = serde_json::from_str(&line).unwrap();
        let mut perfect_matching = PerfectMatching::default();
        for pre_matching in pre_matchings.into_iter() {
            let node = DualNodePtr::new_value(DualNode {
                index: pre_matching.node1 as NodeIndex,
                class: DualNodeClass::DefectVertex {
                    defect_index: pre_matching.vertex1 as VertexIndex,
                },
                defect_size: nonzero::nonzero!(1usize),
                grow_state: DualNodeGrowState::Stay,
                parent_blossom: None,
                dual_variable_cache: (0, 0),
                belonging: belonging.clone(),
            });
            if let Some(node2) = pre_matching.node2 {
                let peer = DualNodePtr::new_value(DualNode {
                    index: node2 as NodeIndex,
                    class: DualNodeClass::DefectVertex {
                        defect_index: pre_matching.vertex2 as VertexIndex,
                    },
                    defect_size: nonzero::nonzero!(1usize),
                    grow_state: DualNodeGrowState::Stay,
                    parent_blossom: None,
                    dual_variable_cache: (0, 0),
                    belonging: belonging.clone(),
                });
                perfect_matching.peer_matchings.push((node, peer));
            } else {
                perfect_matching
                    .virtual_matchings
                    .push((node, pre_matching.vertex2 as VertexIndex));
            }
        }
        perfect_matching
    }
}

// https://stackoverflow.com/questions/30538004/how-do-i-ensure-that-a-spawned-child-process-is-killed-if-my-app-panics
impl Drop for SimulationTcpClient {
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
        if self.sim_config.with_waveform || self.sim_config.dump_debugger_files {
            // only delete binary but keep original waveforms and debugger files
            if !env_is_set("KEEP_RTL_FOLDER") {
                match std::fs::remove_dir_all(format!("../../../simWorkspace/{}/{}/rtl", self.simulation_name, self.name)) {
                    Err(e) => println!("Could not remove rtl folder: {}", e),
                    Ok(_) => println!("Successfully remove rtl folder"),
                }
            }
            match std::fs::remove_dir_all(format!(
                "../../../simWorkspace/{}/{}/verilator",
                self.simulation_name, self.name
            )) {
                Err(e) => println!("Could not remove verilator folder: {}", e),
                Ok(_) => println!("Successfully remove verilator folder"),
            }
        } else {
            match std::fs::remove_dir_all(format!("../../../simWorkspace/{}/{}", self.simulation_name, self.name)) {
                Err(e) => println!("Could not remove build folder: {}", e),
                Ok(_) => println!("Successfully remove build folder"),
            }
        }
    }
}

pub mod simulation_config_default {
    use crate::util::*;
    use std::env;

    pub fn with_waveform() -> bool {
        (cfg!(test) || env_is_set("WITH_WAVEFORM")) && !env_is_set("NO_WAVEFORM")
    }
    pub fn dump_debugger_files() -> bool {
        (cfg!(test) || env_is_set("DUMP_DEBUGGER_FILES")) && !env_is_set("NO_DEBUGGER_FILES")
    }
    pub fn bus_type() -> String {
        env::var("BUS_TYPE").unwrap_or("AxiLite4".to_string())
    }
    pub fn use_64_bus() -> bool {
        env_bool("USE_64_BUS", "USE_32_BUS", true)
    }
    pub fn hard_code_weights() -> bool {
        env_bool("HARD_CODE_WEIGHTS", "DYNAMIC_WEIGHTS", true)
    }
    pub fn support_add_defect_vertex() -> bool {
        env_bool("SUPPORT_ADD_DEFECT_VERTEX", "NO_ADD_DEFECT_VERTEX", true)
    }
    pub fn support_offloading() -> bool {
        env_bool("SUPPORT_OFFLOADING", "NO_OFFLOADING", false)
    }
    pub fn support_layer_fusion() -> bool {
        env_bool("SUPPORT_LAYER_FUSION", "NO_LAYER_FUSION", false)
    }
    pub fn support_load_stall_emulator() -> bool {
        env_bool("SUPPORT_LOAD_STALL_EMULATOR", "NO_LOAD_STALL_EMULATOR", false)
    }
    pub fn inject_registers() -> Vec<String> {
        match env::var("INJECT_REGISTERS") {
            Ok(value) => value.split(',').map(|a| a.to_string()).collect(),
            Err(_) => vec![],
        }
    }
    pub fn context_depth() -> usize {
        env_usize("CONTEXT_DEPTH", 1)
    }
    pub fn broadcast_delay() -> usize {
        env_usize("BROADCAST_DELAY", 0)
    }
    pub fn convergecast_delay() -> usize {
        env_usize("CONVERGECAST_DELAY", 1)
    }
    pub fn conflict_channels() -> usize {
        env_usize("CONFLICT_CHANNELS", 1)
    }
    pub fn clock_divide_by() -> f64 {
        env_f64("CLOCK_DIVIDE_BY", 1.0)
    }
}

impl SimulationConfig {
    pub fn write_to(&self, writer: &mut impl Write) -> std::io::Result<()> {
        let value = serde_json::to_value(self).unwrap();
        let object = value.as_object().unwrap();
        for (key, value) in object {
            write!(writer, "{} = {}\n", key, value)?;
        }
        Ok(())
    }
}
