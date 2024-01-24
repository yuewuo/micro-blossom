use crate::binding::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;

/*
EMBEDDED_BLOSSOM_MAIN=benchmark_primal_simple_match make Xilinx && make -C ../../fpga/Xilinx/VMK180_BRAM
make -C ../../fpga/Xilinx/VMK180_BRAM run_a72
make -C ../../fpga/Xilinx/VMK180_BRAM run_r5
*/

// the value should be 2^k, otherwise it's a lot slower to initialize
// given 256KB memory, maximum size is 256 * 1024 / 34 / 2 = 3855, but other sections may also use some memory
// 3800 guarantees to support up to d=15, but given physical error rate below threshold, it supports d=31 without a problem
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "3600")));

static mut TESTER: UnsafeCell<PrimalSimpleMatch<MAX_NODE_NUM>> = UnsafeCell::new(PrimalSimpleMatch::new());

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

[benchmarker] autotune ... batch size = 17398
[1/3] per_op: 31.93 ns, freq: 31.31530 MHz
[2/3] per_op: 31.94 ns, freq: 31.31297 MHz
[3/3] per_op: 31.94 ns, freq: 31.31258 MHz


R5F:

[benchmarker] autotune ... batch size = 2922
[1/3] per_op: 190.08 ns, freq: 5.26092 MHz
[2/3] per_op: 190.07 ns, freq: 5.26133 MHz
[3/3] per_op: 190.07 ns, freq: 5.26132 MHz

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
