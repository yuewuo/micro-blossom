use crate::binding::*;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;

// the value should be 2^k, because otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "32")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

pub fn main() {
    println!("Benchmark PrimalSimpleMatch");

    let mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> = PrimalSimpleMatch::new();
    for i in 0..10 {
        println!("run benchmark iteration {i}");
        // TODO: start timer
        benchmarker.run(100);
        benchmarker.reset();
        // TODO: end timer
        // TODO: print results
    }
}
