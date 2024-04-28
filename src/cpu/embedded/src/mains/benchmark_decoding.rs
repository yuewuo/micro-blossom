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
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding make aarch64
* simulation (in src/cpu/blossom)
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
* experiment (in this folder)
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

// guarantees decoding up to d=39
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "65536")));

pub const DEFECTS: &'static [u32] = &include_bytes!("./embedded.defects" as u32le);
pub const MAX_CONFLICT_CHANNELS: usize = 8;

static mut PRIMAL_MODULE: UnsafeCell<PrimalModuleEmbedded<MAX_NODE_NUM>> = UnsafeCell::new(PrimalModuleEmbedded::new());
static mut DUAL_MODULE: UnsafeCell<DualModuleStackless<DualDriverTracked<DualDriver<MAX_CONFLICT_CHANNELS>, MAX_NODE_NUM>>> =
    UnsafeCell::new(DualModuleStackless::new(DualDriverTracked::new(DualDriver::new())));

pub fn main() {
    // obtain hardware information
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    assert!(hardware_info.conflict_channels as usize <= MAX_CONFLICT_CHANNELS);
    assert!(hardware_info.conflict_channels >= 1);

    // create primal and dual modules
    let context_id = 0;
    let primal_module = unsafe { PRIMAL_MODULE.get().as_mut().unwrap() };
    // adapt bit width of primal module so that node index will not overflow
    primal_module.nodes.blossom_begin = (1 << hardware_info.vertex_bits) / 2;
    let dual_module = unsafe { DUAL_MODULE.get().as_mut().unwrap() };
    dual_module
        .driver
        .driver
        .reconfigure(hardware_info.conflict_channels, context_id);
    let mut defects_reader = DefectsReader::new(DEFECTS);

    while let Some(defects) = defects_reader.next() {
        if defects.is_empty() {
            continue;
        }
        unsafe { extern_c::clear_instruction_counter() };
        // start timer
        let start = unsafe { extern_c::get_native_time() };
        // reset and load defects
        for (node_index, &vertex_index) in defects.iter().enumerate() {
            dual_module.add_defect(ni!(vertex_index), ni!(node_index));
        }
        // solve it
        let (mut obstacle, _) = dual_module.find_obstacle();
        while !obstacle.is_none() {
            println!("obstacle: {obstacle:?}");
            primal_module.resolve(dual_module, obstacle);
            (obstacle, _) = dual_module.find_obstacle();
        }
        let end = unsafe { extern_c::get_native_time() };
        let counter = unsafe { extern_c::get_instruction_counter() };
        let diff = unsafe { extern_c::diff_native_time(start, end) } as f64;
        println!("[{}] time: {:.3}us, counter: {counter}", defects_reader.count, diff * 1e6);
        primal_module.reset();
        dual_module.reset();
    }
}

/*

Result: handling a conflict takes about 670ns, including computation and the initial transaction.
It seems like the computation is extremely fast (670ns - 590ns = 80ns) per pair of defects.
The most time-consuming operation is reading the bus... about 127ns * 3 = 381ns (according to `test_bram.rs`)
This could potentially be reduced if read operation could happen in a burst? Is this possible in Xilinx library?
This roughly agrees with the results in `benchmark_primal_simple_match.rs`.
The computation time corresponds to only 16 clock cycle at the hardware, and the hardware, if fully pipelined, only requires 5 clock cycles.
It seems like about 3 CPUs could make the FPGA busy.

[974] time: 0.0000005900000132896821, counter: 2
[975] time: 0.000001249999968422344, counter: 5
[976] time: 0.000001249999968422344, counter: 5
[977] time: 0.0000012550000292321783, counter: 5
[978] time: 0.0000005900000132896821, counter: 2
[979] time: 0.0000005900000132896821, counter: 2
[980] time: 0.0000012550000292321783, counter: 5
[981] time: 0.000001249999968422344, counter: 5
[982] time: 0.0000005900000132896821, counter: 2
[983] time: 0.000001259999976355175, counter: 5
[984] time: 0.0000005850000093232666, counter: 2
[985] time: 0.0000005900000132896821, counter: 2
[986] time: 0.0000013899999657951412, counter: 7
[987] time: 0.0000005900000132896821, counter: 2
[988] time: 0.0000005900000132896821, counter: 2
[989] time: 0.0000005900000132896821, counter: 2

*/
