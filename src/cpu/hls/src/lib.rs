use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;

// by default guarantees working at d=15, but can increase if needed
// the value should be a power of 2, because otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(
    option_env!("MAX_NODE_NUM"),
    "512"
)));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

#[repr(C)]
pub struct MinMax {
    pub max: i32,
    pub min: i32,
}

#[no_mangle]
pub unsafe extern "C" fn min_max_rust_idiomatic(numbers: *mut i32, numbers_length: i32) -> MinMax {
    let slice = std::slice::from_raw_parts_mut(numbers, numbers_length as usize);

    slice.iter().fold(MinMax { max: 0, min: 0 }, |mut acc, &x| {
        if x > acc.max {
            acc.max = x;
        }
        if x < acc.min {
            acc.min = x;
        }
        acc
    })
}

#[no_mangle]
pub unsafe extern "C" fn run_benchmark(
    mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
) -> usize {
    benchmarker.clear();
    benchmarker.run(100);
    benchmarker.dual_module.driver.count_set_speed
}

fn main() {
    let mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> =
        PrimalSimpleMatch::new();
    loop {
        benchmarker.run(100);
        benchmarker.clear();
        println!("haha");
    }
}
