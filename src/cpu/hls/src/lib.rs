#![no_std]
#![no_builtins]
#![feature(lang_items)]
#![crate_type = "staticlib"]
// use panic_halt as _;

use micro_blossom_nostd::benchmark::primal_simple_match::*;
use micro_blossom_nostd::blossom_tracker::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;

// by default guarantees working at d=15, but can increase if needed
// the value should be a power of 2, because otherwise it's a lot slower to initialize
// pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(
//     option_env!("MAX_NODE_NUM"),
//     "512"
// )));
pub const MAX_NODE_NUM: usize = 512;
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

// #[repr(C)]
// pub struct MinMax {
//     pub max: i32,
//     pub min: i32,
// }

// #[no_mangle]
// pub unsafe extern "C" fn min_max_rust_idiomatic(numbers: *mut i32, numbers_length: i32) -> MinMax {
//     let slice = std::slice::from_raw_parts_mut(numbers, numbers_length as usize);

//     slice.iter().fold(MinMax { max: 0, min: 0 }, |mut acc, &x| {
//         if x > acc.max {
//             acc.max = x;
//         }
//         if x < acc.min {
//             acc.min = x;
//         }
//         acc
//     })
// }

// #[no_mangle]
// pub unsafe extern "C" fn run_benchmark(
//     mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
// ) -> usize {
//     benchmarker.clear();
//     benchmarker.run(100);
//     benchmarker.dual_module.driver.count_set_speed
// }

// #[no_mangle]
// pub unsafe extern "C" fn run_benchmark() -> usize {
//     0
// }

#[no_mangle]
pub unsafe extern "C" fn run_benchmark(
    blossom_tracker: *mut BlossomTracker<10>,
    node_index: CompactNodeIndex,
) -> usize {
    (*blossom_tracker).create_blossom(node_index);
    (*blossom_tracker).advance_time(20);
    (*blossom_tracker).set_speed(node_index, CompactGrowState::Shrink);
    (*blossom_tracker).get_dual_variable(node_index) as usize
}

// #[inline(never)]
// #[panic_handler]
// fn panic(_info: &core::panic::PanicInfo) -> ! {
//     loop {}
// }

fn main() {
    let mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> =
        PrimalSimpleMatch::new();
    loop {
        benchmarker.run(100);
        benchmarker.clear();
    }
}
