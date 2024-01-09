use crate::binding::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;
use once_cell::unsync::Lazy;

// the value should be 2^k, otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "1024")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

static mut TESTER: UnsafeCell<Lazy<PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>>> =
    UnsafeCell::new(Lazy::new(|| PrimalSimpleMatch::new()));

pub fn main() {
    println!("[benchmark] PrimalSimpleMatch");

    let tester = unsafe { TESTER.get().as_mut().unwrap() };
    // let mut tester: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> = PrimalSimpleMatch::new();
    // tester.reset();

    // // tester.run(50);
    // // tester.reset();
    // let mut benchmarker = Benchmarker::new(|| {
    //     // black_box(tester.run(50));
    //     // black_box(tester.reset());
    // });
    // benchmarker.autotune();
    // benchmarker.run(3);

    for i in 0..10 {
        println!("run benchmark iteration {i}");
        // crate::mains::test_bram::main();
        let start = unsafe { extern_c::test_read32() };
        // println!("timer: {}", unsafe { extern_c::get_native_time() });
        // TODO: start timer
        let inner_repeat = 2000;
        for _ in 0..inner_repeat {
            tester.run(500);
            tester.reset();
        }
        // TODO: end timer
        // TODO: print results
    }
}
