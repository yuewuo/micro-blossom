use crate::binding::*;
use crate::mains::test_get_time::sanity_check as sanity_check_get_time;
use crate::util::*;
use core::hint::black_box;
use micro_blossom_nostd::instruction::*;

/*
 * when building the Vivado project, we need to specify the dual config; also run "make clean" when HDL changes
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_circuit_level_d3.json
 * later on when we only build the Vitis project, there is no need to specify the dual config path
EMBEDDED_BLOSSOM_MAIN=benchmark_reset_speed make Xilinx && make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

pub fn main() {
    println!("Benchmark Reset Speed");

    println!("\n1. Timer Sanity Check");
    sanity_check_get_time();

    println!("\n2. Read Hardware Information");
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    println!("version: {:#08x}", hardware_info.version);
    println!("{hardware_info:#?}");

    println!("\n3. Benchmark Single Context Reset");
    let mut instruction_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), 0)) };
    });
    instruction_benchmarker.autotune();
    instruction_benchmarker.run(3);

    println!("\n4. Benchmark Multi Context Reset");
    let context_count = 32; // note: for systems without such many context, the address is wrapped back
    if hardware_info.context_depth < context_count {
        println!("\n\n\n");
        println!("*******************************");
        println!("[Warning] the benchmark may not work as expected");
        println!(
            "  the actual context depth is {}, smaller than 32",
            hardware_info.context_depth
        );
        println!("  some benchmark may not observe speed up in batch mode");
        println!("*******************************");
        println!("\n\n\n");
    }
    let mut instruction_benchmarker = Benchmarker::new(|| {
        for context_id in 0..context_count as u16 {
            unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), context_id)) };
        }
    });
    instruction_benchmarker.inner_loops = context_count as usize;
    instruction_benchmarker.autotune();
    instruction_benchmarker.run(3);

    let mut head = extern_c::ReadoutHead::new();
    let mut conflicts: [extern_c::ReadoutConflict; 4] = core::array::from_fn(|_| extern_c::ReadoutConflict::invalid());

    println!("\n5. Benchmark Read Obstacle");
    let mut readout_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::get_obstacle(&mut head, conflicts.as_mut_ptr(), 1, 0)) };
    });
    readout_benchmarker.autotune();
    readout_benchmarker.run(3);

    println!("\n6. Benchmark Reset and then Read Obstacle");
    let mut readout_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), 0)) };
        unsafe { black_box(extern_c::get_obstacle(&mut head, conflicts.as_mut_ptr(), 1, 0)) };
    });
    readout_benchmarker.autotune();
    readout_benchmarker.run(3);

    println!("\n7. Benchmark 32 Batch Reset and then Read Obstacle");
    let mut readout_benchmarker = Benchmarker::new(|| {
        for context_id in 0..context_count as u16 {
            unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), context_id)) };
        }
        for context_id in 0..context_count as u16 {
            unsafe { black_box(extern_c::get_obstacle(&mut head, conflicts.as_mut_ptr(), 1, context_id)) };
        }
    });
    readout_benchmarker.inner_loops = context_count as usize;
    readout_benchmarker.autotune();
    readout_benchmarker.run(3);
}
