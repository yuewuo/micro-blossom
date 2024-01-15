use crate::binding::*;
use crate::mains::test_get_time::sanity_check as sanity_check_get_time;
use crate::util::*;
use core::hint::black_box;

/*
 * when building the Vivado project, we need to specify the dual config; also run "make clean" when HDL changes
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom make Xilinx
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_circuit_level_d3.json
 * later on when we only build the Vitis project, there is no need to specify the dual config path
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom make Xilinx && make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

pub fn main() {
    println!("Test MicroBlossom");

    println!("\n1. Timer Sanity Check");
    sanity_check_get_time();

    println!("\n2. Read Hardware Information");
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    println!("version: {:#08x}", hardware_info.version);
    println!("{hardware_info:#?}");

    const TEST_INSTRUCTION: u32 = 0;

    println!("\n3. Test Instruction Counter");
    unsafe { extern_c::clear_instruction_counter() };
    for _ in 0..100 {
        unsafe { extern_c::execute_instruction(TEST_INSTRUCTION, 0) };
    }
    let instruction_counter = unsafe { extern_c::get_instruction_counter() };
    println!("instruction_counter: {instruction_counter}, expected: 100");
    assert_eq!(instruction_counter, 100);

    println!("\n4. Test Instruction Speed");
    let mut instruction_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::execute_instruction(TEST_INSTRUCTION, 0)) };
    });
    instruction_benchmarker.autotune();
    instruction_benchmarker.run(3);
}
