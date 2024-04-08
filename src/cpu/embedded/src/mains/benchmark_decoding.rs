use crate::binding::*;
use crate::defects_reader::*;
use crate::dual_driver::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use include_bytes_plus::include_bytes;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::primal_module_embedded::*;

/*
cp ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.defects ./embedded.defects
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding  make aarch64
# simulation (in src/cpu/blossom)
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
# experiment (in this folder)
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom CLOCK_FREQUENCY=50 DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

// guarantees decoding up to d=39
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "65536")));

cfg_if::cfg_if! {
    if #[cfg(test)] {
        pub const DEFECTS: &'static [u32] = &include_bytes!("./embedded.defects" as u32le);
    } else {
        pub const DEFECTS: &'static [u32] = &[u32::MAX];
    }
}

static mut PRIMAL_MODULE: UnsafeCell<PrimalModuleEmbedded<MAX_NODE_NUM>> = UnsafeCell::new(PrimalModuleEmbedded::new());

pub fn main() {
    let mut defects_reader = DefectsReader::new(DEFECTS);
    let mut primal_module = unsafe { PRIMAL_MODULE.get().as_mut().unwrap() };
    let mut dual_module: DualModuleStackless<DualDriver<1>> = DualModuleStackless::new(DualDriver::new(1, 0));

    // unsafe { black_box(extern_c::execute_instruction(Instruction32::reset().into(), context_id)) };

    while let Some(defects) = defects_reader.next() {}
}
