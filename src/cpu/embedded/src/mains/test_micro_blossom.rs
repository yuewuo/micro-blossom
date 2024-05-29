use crate::binding::*;
use crate::mains::test_get_time::sanity_check as sanity_check_get_time;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
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

 * to run simulation, cd to src/cpu/blossom and run
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom cargo run --release --bin embedded_simulator -- ../../../resources/graphs/example_code_capacity_d3.json
*/

// edge 0 is the edge to be tested
// 1. must be an edge connecting virtual vertices (left, virtual)
// 2. must be the edge that is the minimum-weighted one around the left vertex (to assert for max growable)
pub const EDGE_0_LEFT: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("EDGE_0_LEFT"), "0")));
pub const EDGE_0_VIRTUAL: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("EDGE_0_VIRTUAL"), "1")));
pub const EDGE_0_WEIGHT: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("EDGE_0_WEIGHT"), "2")));

pub const CONTEXT_DEPTH: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("CONTEXT_DEPTH"), "1")));
pub const SUPPORT_OFFLOADING: bool = option_env!("SUPPORT_OFFLOADING").is_some();

pub fn main() {
    println!("Test MicroBlossom");
    let cid = CONTEXT_DEPTH as u16 - 1; // always test the largest index of context
    let left = ni!(EDGE_0_LEFT);
    let node = ni!(0);
    let weight = EDGE_0_WEIGHT as u16;
    println!("context id: {cid}, left: {left} (non-virtual), right: {EDGE_0_VIRTUAL} (virtual), edge weight: {weight}");
    println!("support_offloading: {SUPPORT_OFFLOADING}");

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
        unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), cid)) };
        unsafe { extern_c::set_maximum_growth(0, cid) }; // disable automatic growth
    }
    let instruction_counter = unsafe { extern_c::get_instruction_counter() };
    println!("instruction_counter: {instruction_counter}, expected: {test_count}");
    assert_eq!(instruction_counter, test_count);

    println!("\n4. Test Max Growable Fetch");
    println!("reset");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), cid) };
    println!("get obstacle");
    let readout = unsafe { extern_c::get_single_readout(cid) };
    println!("readout: {:#?}", readout);
    assert_eq!(readout.max_growable, u8::MAX); // because there is no defect yet
    println!("add defect");
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(left, node).into(), cid) };
    println!("get obstacle");
    let readout = unsafe { extern_c::get_single_readout(cid) };
    println!("readout: {:#?}", readout);
    assert_eq!(readout.max_growable, EDGE_0_WEIGHT as u8); // because there is no defect yet

    println!("\n5. Test Grow and Obstacle Detection");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), cid) };
    unsafe { extern_c::set_maximum_growth(0, cid) }; // disable primal offloading growth
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(left, node).into(), cid) };
    let readout = unsafe { extern_c::get_single_readout(cid) };
    // println!("head: {head:#?}, conflicts: {conflicts:#?}");
    assert_eq!(readout.max_growable as u16, weight);
    assert!(!readout.has_conflict());
    unsafe { extern_c::execute_instruction(Instruction32::grow(weight as CompactWeight).into(), cid) };
    let readout = unsafe { extern_c::get_single_readout(cid) };
    // println!("head: {head:#?}, conflicts: {conflicts:#?}");
    if SUPPORT_OFFLOADING {
        assert_eq!(readout.max_growable, u8::MAX, "should be offloaded");
        assert!(!readout.has_conflict());
    } else {
        assert_eq!(readout.max_growable, 0);
        assert!(readout.has_conflict());
        unsafe { extern_c::execute_instruction(Instruction32::set_speed(node, CompactGrowState::Stay).into(), cid) };
        let readout = unsafe { extern_c::get_single_readout(cid) };
        // println!("head: {head:#?}, conflicts: {conflicts:#?}");
        assert_eq!(readout.max_growable, u8::MAX);
        assert!(!readout.has_conflict());
    }

    println!("\n6. Test Setting Maximum Growth");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), cid) };
    unsafe { extern_c::set_maximum_growth(100, cid) };
    let readout = unsafe { extern_c::get_single_readout(cid) };
    println!("readout: {readout:#?}");
    assert_eq!(unsafe { extern_c::get_maximum_growth(cid) }, 100);
    unsafe { extern_c::set_maximum_growth(200, cid) };
    unsafe { extern_c::get_single_readout(cid) };
    assert_eq!(unsafe { extern_c::get_maximum_growth(cid) }, 200);
    unsafe { extern_c::set_maximum_growth(0, cid) }; // set it back to 0 before doing other operations, to avoid data race

    println!("\n7. Test Primal Offloading Growth");
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), cid) };
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(left, node).into(), cid) };
    unsafe { extern_c::set_maximum_growth(weight + 10, cid) };
    let readout = unsafe { extern_c::get_single_readout(cid) };
    println!("readout: {readout:#?}");
    if SUPPORT_OFFLOADING {
        assert_eq!(readout.max_growable, u8::MAX);
        assert!(!readout.has_conflict());
        assert_eq!(
            readout.accumulated_grown, weight,
            "the primal offloading grow unit should have grown by weight"
        );
    } else {
        assert_eq!(readout.max_growable, 0);
        assert_eq!(
            readout.accumulated_grown, weight,
            "the primal offloading grow unit should have grown by weight"
        );
        unsafe { extern_c::set_maximum_growth(weight + 10, cid) };
        let readout = unsafe { extern_c::get_single_readout(cid) };
        assert_eq!(readout.accumulated_grown, 0, "should be automatically cleared");
        unsafe { extern_c::set_maximum_growth(0, cid) };
    }

    println!("\n8. Test Set Speed");
    unsafe { extern_c::execute_instruction(Instruction32::set_speed(node, CompactGrowState::Stay).into(), cid) };
    let readout = unsafe { extern_c::get_single_readout(cid) };
    println!("readout: {readout:#?}");
    assert_eq!(readout.max_growable, u8::MAX);

    println!("\n9. Test Context Switching");
    let cid_1 = 0;
    let cid_2 = CONTEXT_DEPTH as u16 - 1;
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), cid_1) };
    unsafe { extern_c::execute_instruction(Instruction32::reset().into(), cid_2) };
    unsafe { extern_c::set_maximum_growth(0, cid_1) };
    unsafe { extern_c::set_maximum_growth(0, cid_2) };
    unsafe { extern_c::execute_instruction(Instruction32::add_defect_vertex(left, node).into(), cid_1) };
    let readout = unsafe { extern_c::get_single_readout(cid_1) };
    assert_eq!(readout.max_growable as u16, weight, "context 1 should detect growth");
    let readout = unsafe { extern_c::get_single_readout(cid_2) };
    if CONTEXT_DEPTH == 1 {
        assert_eq!(readout.max_growable as u16, weight, "context 2 should wrap up and see it");
    } else {
        assert_eq!(readout.max_growable, u8::MAX, "context 2 should not see it");
    }
}
