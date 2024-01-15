use crate::binding::*;
use crate::mains::test_get_time::sanity_check as sanity_check_get_time;
use crate::util::*;
use core::hint::black_box;
use core::sync::atomic::{compiler_fence, Ordering};

/*
 * when building the Vivado project, we need to specify the dual config
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom make Xilinx
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_circuit_level_d3.json
 * later on when we only build the Vitis project, there is no need to specify the dual config path
EMBEDDED_BLOSSOM_MAIN=test_micro_blossom make Xilinx && make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

pub fn main() {
    println!("Test MicroBlossom");

    println!("\n1. Timer Sanity Check");
    sanity_check_get_time();
}
