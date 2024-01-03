#![no_std]
#![no_main]

use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;

// by default guarantees working at d=15, but can increase if needed
// the value should be a power of 2, because otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "32")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

use panic_halt as _;

#[no_mangle]
pub extern "C" fn benchmark() {
    let mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> = PrimalSimpleMatch::new();
    loop {
        benchmarker.run(10);
        benchmarker.reset();
    }
}
