use crate::binding::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;

// the value should be 2^k, otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "1024")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

static mut TESTER: UnsafeCell<PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>> =
    UnsafeCell::new(PrimalSimpleMatch::new());

pub fn main() {
    println!("[benchmark] PrimalSimpleMatch");

    let tester = unsafe { TESTER.get().as_mut().unwrap() };

    let mut benchmarker = Benchmarker::new(|| {
        black_box(tester.run(MAX_NODE_NUM / 2));
        black_box(tester.reset());
    });
    benchmarker.inner_loops = MAX_NODE_NUM / 2; // how many obstacles reported
    benchmarker.autotune();
    benchmarker.run(3);
}

/*

Results: A72 is significantly faster than R5F

A72:

[benchmarker] automatic batch size = 7964
[1/3] per_op: 245.22 ns, freq: 4.07797 MHz
[2/3] per_op: 245.20 ns, freq: 4.07823 MHz
[3/3] per_op: 245.20 ns, freq: 4.07823 MHz


R5F:

[benchmarker] automatic batch size = 64414
[1/3] per_op: 30.57 ns, freq: 32.70786 MHz
[2/3] per_op: 30.57 ns, freq: 32.71084 MHz
[3/3] per_op: 30.57 ns, freq: 32.71084 MHz

*/

/*

debug note 2024.1.9: the problem seems to be memory region overlap!

When allocating on stack, MAX_NODE_NUM = 50, it all seems fine;
                          MAX_NODE_NUM = 64, the timer value are all 0 and even stuck at A72 core;
Note that each element takes 32+2 bytes, so 128 elements take 4352 bytes, exceeding the 4KB stack space.

Although I don't understand how `static mut UnsafeCell` would place the variable, it seems to be in the `.data` segment.
If loading the ELF results in overlapping in the .data segment, then it could trigger similar effects.
The triggering MAX_NODE_NUM, however, is different. Now 110 is safe but 120 fails: the time values are all 0.
This corresponds to 8KB space, but why `.data` segment would be limited anyway?

How can I debug this?

1. The problem seems to be clear: it's due to memory overlapping.
2. even though the data is allocated globally, when initializing it, the program still runs in stack!!!!

One solution is to increase the size of the stack, or to use some uninitialized memory structure.

The more elegent solution (the final solution) is to use const function in Rust to avoid runtime initialization.
By marking the primal module's `new` functions as const function, it can be evaluated and initialized by compiler.

*/
