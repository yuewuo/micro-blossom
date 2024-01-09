use crate::binding::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;
use once_cell::unsync::Lazy;

// the value should be 2^k, otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "128")));
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
        println!("timer: {}", unsafe { extern_c::get_native_time() });
        // TODO: start timer
        let inner_repeat = 2000;
        for _ in 0..inner_repeat {
            tester.run(MAX_NODE_NUM / 2);
            // println!("count_set_speed: {}", tester.dual_module.driver.count_set_speed);
            // println!("count_set_blossom: {}", tester.dual_module.driver.count_set_blossom);
            tester.reset();
        }
        // TODO: end timer
        // TODO: print results
    }
}

/*

debug note: the problem seems to be memory region overlap!

When allocating on stack, MAX_NODE_NUM = 50, it all seems fine;
                          MAX_NODE_NUM = 64, the time value are all 0;
Note that each element takes 32+2 bytes, so 128 elements take 4352 bytes, exceeding the 4KB stack space.

Although I don't understand how `static mut UnsafeCell` would place the variable, it seems to be in the `.data` segment.
If loading the ELF results in overlapping in the .data segment, then it could trigger similar effects.
The triggering MAX_NODE_NUM, however, is different. Now 110 is safe but 120 fails: the time values are all 0.
This corresponds to 8KB space, but why `.data` segment would be limited anyway?

How can I debug this?

1. The problem seems to be clear: it's due to memory overlapping.
2. even though the data is allocated globally, when initializing it, the program still runs in stack!!!!

The solution is to increase the size of the stack, or to use some uninitialized memory structure.
*/
