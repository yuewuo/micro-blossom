use crate::binding::*;
use core::cell::UnsafeCell;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;
use once_cell::unsync::Lazy;

// the value should be 2^k, otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "1024")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

static mut BENCHMARKER: UnsafeCell<Lazy<PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>>> =
    UnsafeCell::new(Lazy::new(|| PrimalSimpleMatch::new()));

pub fn main() {
    println!("[benchmark] PrimalSimpleMatch");

    let benchmarker = unsafe { BENCHMARKER.get().as_mut().unwrap() };
    for i in 0..10 {
        println!("run benchmark iteration {i}");
        // TODO: start timer
        let inner_repeat = 2000;
        for _ in 0..inner_repeat {
            benchmarker.run(512);
            benchmarker.reset();
        }
        // TODO: end timer
        // TODO: print results
    }
}
