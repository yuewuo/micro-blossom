use crate::binding::*;
use crate::extern_c::*;
use crate::mains::test_get_time::sanity_check as sanity_check_get_time;
use crate::util::*;
use core::hint::black_box;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::util::*;

/*

We benchmark the elementary operations that are useful to do a cycle-accurate simulation in software

 * when building the Vivado project, we need to specify the dual config; also run "make clean" when HDL changes
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
EMBEDDED_BLOSSOM_MAIN=benchmark_axi4_dual make Xilinx
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
 * later on when we only build the Vitis project, there is no need to specify the dual config path
EMBEDDED_BLOSSOM_MAIN=benchmark_axi4_dual make Xilinx && make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

fn reset() {
    unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), 0)) };
}

pub fn main() {
    println!("Benchmark Reset Speed");

    println!("\n1. Timer Sanity Check");
    sanity_check_get_time();

    println!("\n2. Read Hardware Information");
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    println!("version: {:#08x}", hardware_info.version);
    println!("{hardware_info:#?}");

    // println!("\n3. Benchmark Single Context Reset");
    // let mut instruction_benchmarker = Benchmarker::new(|| {
    //     unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), 0)) };
    // });
    // instruction_benchmarker.autotune();
    // instruction_benchmarker.run(3);

    let mut readout = SingleReadout::default();

    // when there is no conflict, the function usually runs faster because only one 64 bit read is performed
    println!("\n4. Benchmark Read Obstacle No Conflict");
    reset();
    let mut readout_benchmarker = Benchmarker::new(|| {
        readout = unsafe { black_box(extern_c::get_single_readout(0)) };
    });
    readout_benchmarker.autotune();
    readout_benchmarker.run(3);
    println!("readout: {readout:?}");

    // when there is conflict, the function runs slower
    println!("\n5. Benchmark Read Obstacle With Conflict");
    reset();
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(ni!(0), ni!(0)).into(), 0) };
    unsafe { extern_c::execute_instruction(Instruction32::grow(2 as CompactWeight).into(), 0) };
    let mut readout_benchmarker = Benchmarker::new(|| {
        readout = unsafe { black_box(extern_c::get_single_readout(0)) };
    });
    readout_benchmarker.autotune();
    readout_benchmarker.run(3);
    println!("head: {readout:?}");
}

/*

Optimization of reading obstacle: read 256 bit memory and then calculate locally, to let the hardware use memory burst
before: read no conflict: 235ns, read conflict: 500.00 ns
after: read no conflict: 229ns, read with conflict: 340ns

*/
