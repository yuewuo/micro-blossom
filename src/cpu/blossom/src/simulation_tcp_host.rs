use crate::dual_module_adaptor::*;
use crate::resources::*;
use crate::util::*;
use derivative::Derivative;
use embedded_blossom::extern_c::*;
use embedded_blossom::util::*;
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
use std::env;
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::process::Child;

pub const MAX_CONFLICT_CHANNELS: usize = 15;

#[derive(Serialize, Derivative, Clone)]
#[derivative(Default)]
pub struct SimulationConfig {
    #[derivative(Default(value = "simulation_config_default::with_waveform()"))]
    pub with_waveform: bool,
    #[derivative(Default(value = "simulation_config_default::dump_debugger_files()"))]
    pub dump_debugger_files: bool,
    #[derivative(Default(value = "simulation_config_default::bus_type()"))]
    pub bus_type: String,
    #[derivative(Default(value = "simulation_config_default::use_64_bus()"))]
    pub use_64_bus: bool,
    #[derivative(Default(value = "env_usize(\"CONTEXT_DEPTH\", 1)"))]
    pub context_depth: usize,
    #[derivative(Default(value = "env_usize(\"BROADCAST_DELAY\", 0)"))]
    pub broadcast_delay: usize,
    #[derivative(Default(value = "env_usize(\"CONVERGECAST_DELAY\", 0)"))]
    pub convergecast_delay: usize,
    #[derivative(Default(value = "env_usize(\"CONFLICT_CHANNELS\", 1)"))]
    pub conflict_channels: usize,
    #[derivative(Default(value = "simulation_config_default::hard_code_weights()"))]
    pub hard_code_weights: bool,
    #[derivative(Default(value = "simulation_config_default::support_add_defect_vertex()"))]
    pub support_add_defect_vertex: bool,
    #[derivative(Default(value = "simulation_config_default::support_offloading()"))]
    pub support_offloading: bool,
    #[derivative(Default(value = "simulation_config_default::support_layer_fusion()"))]
    pub support_layer_fusion: bool,
    #[derivative(Default(value = "simulation_config_default::inject_registers()"))]
    pub inject_registers: Vec<String>,
    #[derivative(Default(value = "env_usize(\"CLOCK_DIVIDE_BY\", 1)"))]
    pub clock_divide_by: usize,
}

pub struct SimulationTcpClient<const SIMULATION_NAME: &'static str> {
    pub link: Mutex<Link>,
    pub name: String,
    pub context_id: u16,
    pub maximum_growth: Vec<u16>,
    pub simulation_duration: Duration,
    pub sim_config: SimulationConfig,
}

pub struct Link {
    pub port: u16,
    pub child: Child,
    pub reader: BufReader<TcpStream>,
    pub writer: LineWriter<TcpStream>,
}

impl<const SIMULATION_NAME: &'static str> SimulationTcpClient<SIMULATION_NAME> {
    pub fn new_with_name_raw(
        micro_blossom: MicroBlossomSingle,
        name: String,
        sim_config: SimulationConfig,
    ) -> std::io::Result<Self> {
        let hostname = "127.0.0.1";
        let listener = TcpListener::bind(format!("{hostname}:0"))?;
        let port = listener.local_addr()?.port();
        // start the scala simulator host
        println!("Starting Scala simulator host... this may take a while (listening on {hostname}:{port})");
        let child = SCALA_MICRO_BLOSSOM_RUNNER.run(
            format!("microblossom.{SimulationName}"),
            [format!("{hostname}"), format!("{port}"), format!("{name}")],
        )?;
        let (socket, _addr) = listener.accept()?;
        let mut reader = BufReader::new(socket.try_clone()?);
        let mut writer = LineWriter::new(socket);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        assert_eq!(
            line,
            format!("{SimulationName} v0.0.1, ask for decoding graph\n"),
            "handshake error"
        );
        write!(writer, "{}\n", serde_json::to_string(&micro_blossom).unwrap())?;
        let simulation_lock = SCALA_SIMULATION_LOCK.lock();
        sim_config.write_to(&mut writer)?;
        line.clear();
        reader.read_line(&mut line)?;
        assert_eq!(line, "simulation started\n");
        drop(simulation_lock);
        assert!(sim_config.conflict_channels <= MAX_CONFLICT_CHANNELS);
        let conflict_channels = sim_config.conflict_channels;
        let mut conflicts_store = ConflictsStore::new();
        conflicts_store.reconfigure(conflict_channels as u8);
        let mut value = Self {
            name,
            context_id: 0,
            maximum_growth: vec![0; sim_config.context_depth],
            sim_config,
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

    pub fn new_with_name(initializer: &SolverInitializer, name: String) -> std::io::Result<Self> {
        // in simulation, positions doesn't matter because it's not going to affect the timing constraint
        Self::new_with_name_raw(
            MicroBlossomSingle::new_initializer_only(initializer),
            name,
            Default::default(),
        )
    }

    pub fn new(initializer: &SolverInitializer) -> std::io::Result<Self> {
        let name = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        Self::new_with_name(initializer, name)
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
        if self.sim_config.use_64_bus {
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
    pub fn inject_registers() -> Vec<String> {
        match env::var("INJECT_REGISTERS") {
            Ok(value) => value.split(',').map(|a| a.to_string()).collect(),
            Err(_) => vec![],
        }
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
