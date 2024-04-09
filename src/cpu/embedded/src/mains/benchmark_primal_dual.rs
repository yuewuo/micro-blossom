use crate::binding::*;
use crate::dual_driver::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;

/*
EMBEDDED_BLOSSOM_MAIN=benchmark_primal_dual make aarch64
* simulation (in src/cpu/blossom)
EMBEDDED_BLOSSOM_MAIN=benchmark_primal_dual WITH_WAVEFORM=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
* experiment (in this folder)
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

// guarantees decoding up to d=39
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "65536")));

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
    let dual_module = unsafe { DUAL_MODULE.get().as_mut().unwrap() };
    dual_module
        .driver
        .driver
        .reconfigure(hardware_info.conflict_channels, context_id);

    primal_module.reset();
    dual_module.reset();

    println!("\n1. Primal Reset");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(primal_module.reset());
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n2. Dual Reset");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.reset());
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n3. Dual Reset + Add Defect");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.add_defect(ni!(0), ni!(0)));
        black_box(dual_module.add_defect(ni!(1), ni!(1)));
        dual_module.reset();
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n4. Dual Reset + Add Defect + Find Obstacle");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.add_defect(ni!(0), ni!(0)));
        black_box(dual_module.add_defect(ni!(1), ni!(1)));
        dual_module.find_obstacle();
        dual_module.reset();
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n5. Dual Reset + Add Defect + Find Obstacle + Primal Resolve");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.add_defect(ni!(0), ni!(0)));
        black_box(dual_module.add_defect(ni!(1), ni!(1)));
        let (obstacle, _) = dual_module.find_obstacle();
        primal_module.resolve(dual_module, obstacle);
        dual_module.reset();
        primal_module.reset();
    });
    benchmarker.autotune();
    benchmarker.run(3);
}

/*
1. Primal Reset
[benchmarker] autotune ... batch size = 692480190
[1/3] per_op: 1.44 ns, freq: 692.48040 MHz
[2/3] per_op: 1.44 ns, freq: 692.48040 MHz
[3/3] per_op: 1.44 ns, freq: 692.48040 MHz

2. Dual Reset
[benchmarker] autotune ... batch size = 19718281
[1/3] per_op: 51.43 ns, freq: 19.44444 MHz
[2/3] per_op: 51.43 ns, freq: 19.44444 MHz
[3/3] per_op: 50.71 ns, freq: 19.71831 MHz

3. Dual Reset + Add Defect
[benchmarker] autotune ... batch size = 10523192
[1/3] per_op: 95.71 ns, freq: 10.44776 MHz
[2/3] per_op: 95.03 ns, freq: 10.52320 MHz
[3/3] per_op: 95.71 ns, freq: 10.44776 MHz

4. Dual Reset + Add Defect + Find Obstacle
[benchmarker] autotune ... batch size = 1492521
[1/3] per_op: 670.00 ns, freq: 1.49253 MHz
[2/3] per_op: 670.01 ns, freq: 1.49252 MHz
[3/3] per_op: 670.01 ns, freq: 1.49252 MHz

5. Dual Reset + Add Defect + Find Obstacle + Primal Resolve
[benchmarker] autotune ... batch size = 1257809
[1/3] per_op: 795.04 ns, freq: 1.25780 MHz
[2/3] per_op: 795.02 ns, freq: 1.25783 MHz
[3/3] per_op: 795.03 ns, freq: 1.25781 MHz
[exit]

*/
