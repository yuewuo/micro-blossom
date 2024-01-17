//! # Embedded Simulator
//!
//! This simulator uses the DualModuleAxi4Driver to provide the underlying C functions for the embedded main function.
//!
//! ## Examples
//!
//! First generate the resources using `cargo run --bin micro-blossom`.
//!
//! ```sh
//! cargo run --release --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_planar_d3.json
//! EMBEDDED_BLOSSOM_MAIN=test_get_time cargo run --release --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_planar_d3.json  # note: it's normal that sleep() will take almost forever
//!
//! EMBEDDED_BLOSSOM_MAIN=benchmark_reset_speed cargo run --release --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_planar_d3.json
//! gtkwave ../../../simWorkspace/MicroBlossomHost/benchmark_reset_speed/hosted.fst
//!
//! EMBEDDED_BLOSSOM_MAIN=benchmark_primal_simple_match cargo --release run --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_planar_d3.json
//! ```
//!

use clap::Parser;
use cty::c_char;
use embedded_blossom::extern_c::MicroBlossomHardwareInfo;
use embedded_blossom::{rust_main_raw, RUST_MAIN_NAME};
use lazy_static::lazy_static;
use micro_blossom::dual_module_axi4::*;
use micro_blossom::resources::MicroBlossomSingle;
use micro_blossom_nostd::instruction::Instruction32;
use parking_lot::Mutex;
use std::fs;
use std::time::Instant;

// assume 200 MHz clock
const MICRO_BLOSSOM_FREQUENCY: f64 = 200e6;
const CONSIDER_CPU_TIME: bool = true;

#[derive(Parser, Clone)]
#[clap(author = clap::crate_authors!(", "))]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "Micro Blossom Embedded Simulator (Verilog)")]
#[clap(color = clap::ColorChoice::Auto)]
#[clap(propagate_version = true)]
#[clap(arg_required_else_help = true)]
pub struct EmbeddedSimulator {
    /// code distance
    #[clap(value_parser)]
    micro_blossom_graph_path: String,
}

impl EmbeddedSimulator {
    pub fn run(&self) {
        let _ = BEGIN_TIME.elapsed(); // must access first to initialize it
        let micro_blossom_json = fs::read_to_string(self.micro_blossom_graph_path.clone()).unwrap();
        let micro_blossom: MicroBlossomSingle = serde_json::from_str(micro_blossom_json.as_str()).unwrap();
        {
            let mut driver = SIMULATOR_DRIVER.lock();
            assert!(driver.is_none(), "EmbeddedSimulator::run should not be executed twice");
            let _ = driver
                .insert(DualModuleAxi4Driver::new_with_name_raw(micro_blossom, RUST_MAIN_NAME.to_string(), true).unwrap());
        }
        get_native_time();
        rust_main_raw();
        SIMULATOR_DRIVER.lock().take(); // drop the connection
    }
}

fn main() {
    EmbeddedSimulator::parse().run();
}

#[no_mangle]
extern "C" fn set_leds(mask: u32) {
    println!("[set_leds] mask = {mask} = {mask:#b}");
}

#[no_mangle]
extern "C" fn print_char(c: c_char) {
    print!("{}", (c as u8) as char);
}

#[no_mangle]
extern "C" fn test_write32(_value: u32) {
    unimplemented!()
}

#[no_mangle]
extern "C" fn test_read32() -> u32 {
    unimplemented!()
}

lazy_static! {
    static ref BEGIN_TIME: Instant = Instant::now();
    static ref SIMULATOR_DRIVER: Mutex<Option<DualModuleAxi4Driver>> = Mutex::new(None);
}

// #[no_mangle]
// extern "C" fn get_native_time() -> u64 {
//     BEGIN_TIME.elapsed().as_nanos() as u64
// }

#[no_mangle]
extern "C" fn get_native_time() -> u64 {
    let mut locked = SIMULATOR_DRIVER.lock();
    let driver = locked.as_mut().unwrap();
    let nanos = driver.memory_read_64(0).unwrap() as f64 / MICRO_BLOSSOM_FREQUENCY * 1e9;
    if CONSIDER_CPU_TIME {
        nanos.round() as u64 + ((BEGIN_TIME.elapsed().as_nanos() - driver.simulation_duration.as_nanos()) as u64)
    } else {
        nanos.round() as u64
    }
}

#[no_mangle]
extern "C" fn diff_native_time(start: u64, end: u64) -> f32 {
    (end - start) as f32 * 1.0e-9
}

#[no_mangle]
extern "C" fn get_hardware_info() -> MicroBlossomHardwareInfo {
    SIMULATOR_DRIVER.lock().as_mut().unwrap().get_hardware_info()
}

#[no_mangle]
extern "C" fn execute_instruction(instruction: u32, context_id: u16) {
    SIMULATOR_DRIVER
        .lock()
        .as_mut()
        .unwrap()
        .execute_instruction(Instruction32(instruction), context_id)
}
