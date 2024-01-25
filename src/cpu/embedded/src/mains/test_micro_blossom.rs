use crate::binding::*;
use crate::mains::test_get_time::sanity_check as sanity_check_get_time;
use core::hint::black_box;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::util::*;

/*
 * when building the Vivado project, we need to specify the dual config; also run "make clean" when HDL changes
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom make Xilinx
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom CLOCK_FREQUENCY=50 DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
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

    println!("\n3. Test Instruction Counter");
    unsafe { extern_c::clear_instruction_counter() };
    let test_count = 15;
    for _ in 0..test_count {
        unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), 0)) };
        unsafe { extern_c::set_maximum_growth(0, 0) }; // disable automatic growth
    }
    let instruction_counter = unsafe { extern_c::get_instruction_counter() };
    println!("instruction_counter: {instruction_counter}, expected: {test_count}");
    assert_eq!(instruction_counter, test_count);

    println!("\n4. Test Max Growable Fetch");
    let mut head = extern_c::ReadoutHead::new();
    let mut conflicts: [extern_c::ReadoutConflict; 1] = core::array::from_fn(|_| extern_c::ReadoutConflict::invalid());
    for nop in [0, 30] {
        println!("  [nop = {nop}]");
        // nop to reduce the read halt of same context
        println!("    reset");
        unsafe { extern_c::execute_instruction(Instruction32::reset().into(), 0) };
        for _ in 0..nop {
            if cfg!(feature = "tiny_benchmark_time") {
                unsafe { extern_c::execute_instruction(Instruction32::reserved().into(), 1) };
            } else {
                nop_delay(10);
            }
        }
        println!("    get obstacle");
        unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
        println!("head: {head:#?}");
        assert_eq!(head.growable, u16::MAX); // because there is no defect yet
        println!("    add defect");
        unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(ni!(1), ni!(0)).into(), 0) };
        for _ in 0..nop {
            if cfg!(feature = "tiny_benchmark_time") {
                unsafe { extern_c::execute_instruction(Instruction32::reserved().into(), 1) };
            } else {
                nop_delay(10);
            }
        }
        println!("    get obstacle");
        unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
        println!("head: {head:#?}");
        // println!("conflicts: {conflicts:#?}");
        assert_eq!(head.growable, 2);
    }

    println!("\n5. Test Grow and Obstacle Detection");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), 0) };
    unsafe { extern_c::set_maximum_growth(0, 0) }; // disable primal offloading growth
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(ni!(1), ni!(0)).into(), 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
    // println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(head.growable, 2);
    assert_eq!(conflicts[0].valid, 0);
    unsafe { extern_c::execute_instruction(Instruction32::grow(2).into(), 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
    // println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(head.growable, 0);
    assert_eq!(conflicts[0].valid, 1);
    unsafe { extern_c::execute_instruction(Instruction32::set_speed(ni!(0), CompactGrowState::Stay).into(), 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
    // println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(head.growable, u16::MAX);
    assert_eq!(conflicts[0].valid, 0);

    println!("\n6. Test Setting Maximum Growth");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), 0) };
    unsafe { extern_c::set_maximum_growth(100, 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 0, 0) };
    // println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(head.maximum_growth, 100);
    unsafe { extern_c::set_maximum_growth(200, 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 0, 0) };
    assert_eq!(head.maximum_growth, 200);
    unsafe { extern_c::set_maximum_growth(0, 0) }; // set it back to 0 before doing other operations, to avoid data race

    println!("\n7. Test Primal Offloading Growth");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), 0) };
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(ni!(1), ni!(0)).into(), 0) };
    unsafe { extern_c::set_maximum_growth(10, 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
    println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(head.growable, 0);
    assert_eq!(
        head.accumulated_grown, 2,
        "the primal offloading grow unit should have grown by 2"
    );
    unsafe { extern_c::set_maximum_growth(10, 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
    assert_eq!(head.accumulated_grown, 0, "automatic clear");
    unsafe { extern_c::set_maximum_growth(0, 0) };

    println!("\n8. Test Set Speed");
    unsafe { extern_c::execute_instruction(Instruction32::set_speed(ni!(0), CompactGrowState::Stay).into(), 0) };
    unsafe { extern_c::get_conflicts(&mut head, conflicts.as_mut_ptr(), 1, 0) };
    println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(head.growable, u16::MAX);
}
