#![no_std]
#![no_main]

use core::arch::asm;
use embedded_blossom_riscv as _; // import panic handler
use embedded_blossom_riscv::riscv_driver::RiscVCommandDriver;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;
use riscv_rt::entry;

// by default guarantees working at d=15, but can increase if needed
// the value should be a power of 2, because otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "32")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

pub const RISCV_DRIVER_BASE_ADDRESS: usize = 0xF2000000;

fn delay(cycles: u32) {
    for _ in 0..cycles {
        unsafe {
            asm!("nop");
        }
    }
}

fn set_leds(mask: u32) {
    unsafe {
        *(0xF0000000 as *mut u32) = mask;
    }
}

fn test_acc(mask: u32) {
    unsafe {
        // currently it's a mock accelerator of 4kB memory; just to test assertion
        *(0xF1000000 as *mut u32) = mask;
        core::assert_eq!(*(0xF1000000 as *const u32), mask);
    }
}

#[entry]
fn main() -> ! {
    set_leds(0x00);
    let mut mask = 0x40;
    let mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> = PrimalSimpleMatch::new();
    loop {
        set_leds(mask);
        test_acc(mask);
        mask >>= 1;
        if mask == 0 {
            mask = 0x40;
        }
        // delay(300000);
        // delay(100); // for simulation
        benchmarker.run(10);
        benchmarker.reset();
    }
}
