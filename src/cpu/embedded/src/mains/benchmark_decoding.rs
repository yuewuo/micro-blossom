use crate::binding::*;
use crate::defects_reader::*;
use crate::dual_driver::*;
use core::cell::UnsafeCell;
use include_bytes_plus::include_bytes;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;

/*
cp ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.defects ./embedded.defects
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding  make aarch64
* simulation (in src/cpu/blossom)
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
* experiment (in this folder)
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom CLOCK_FREQUENCY=50 DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

// guarantees decoding up to d=39
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "65536")));

pub const DEFECTS: &'static [u32] = &include_bytes!("./embedded.defects" as u32le);
pub const MAX_CONFLICT_CHANNELS: usize = 8;
pub const MAX_ITERATION: usize = 65536;

static mut PRIMAL_MODULE: UnsafeCell<PrimalModuleEmbedded<MAX_NODE_NUM>> = UnsafeCell::new(PrimalModuleEmbedded::new());

pub fn main() {
    // obtain hardware information
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    assert!(hardware_info.conflict_channels as usize <= MAX_CONFLICT_CHANNELS);
    assert!(hardware_info.conflict_channels >= 1);

    // create primal and dual modules
    let context_id = 0;
    let primal_module = unsafe { PRIMAL_MODULE.get().as_mut().unwrap() };
    let mut dual_module: DualModuleStackless<DualDriverTracked<DualDriver<MAX_CONFLICT_CHANNELS>, MAX_NODE_NUM>> =
        DualModuleStackless::new(DualDriverTracked::new(DualDriver::new(
            hardware_info.conflict_channels,
            context_id,
        )));
    let mut defects_reader = DefectsReader::new(DEFECTS);

    while let Some(defects) = defects_reader.next() {
        // reset and load defects
        primal_module.reset();
        dual_module.reset();
        for (node_index, &vertex_index) in defects.iter().enumerate() {
            dual_module.add_defect(ni!(vertex_index), ni!(node_index));
        }
        // start timer
        let start = unsafe { extern_c::get_native_time() };
        let (mut obstacle, _) = dual_module.find_obstacle();
        let mut iteration = 0;
        while !obstacle.is_none() && iteration < MAX_ITERATION {
            iteration += 1;
            println!("obstacle: {obstacle:?}");
            debug_assert!(
                obstacle.is_obstacle(),
                "dual module should spontaneously process all finite growth"
            );
            primal_module.resolve(&mut dual_module, obstacle);
            (obstacle, _) = dual_module.find_obstacle();
        }
        if iteration == MAX_ITERATION {
            println!("[error] max iteration reached, check for infinite loop");
            panic!()
        }
        let end = unsafe { extern_c::get_native_time() };
        let diff = unsafe { extern_c::diff_native_time(start, end) } as f64;
        println!("[{}] time: {diff}", defects_reader.count);
    }
}
