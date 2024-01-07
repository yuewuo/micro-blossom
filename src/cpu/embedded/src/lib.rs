#![no_std]

pub mod binding;
#[cfg(feature = "riscv")]
pub mod riscv_driver;

pub use binding::*;
use core::arch::asm;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::benchmark::primal_simple_match::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;
use panic_halt as _;

#[cfg(feature = "riscv")]
use riscv_driver::*;

// by default guarantees working at d=15, but can increase if needed
// the value should be a power of 2, because otherwise it's a lot slower to initialize
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "32")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    println!("[main] embedded blossom");

    println!("Hello, world!");
    println!("My value is {}", 666);

    unsafe {
        extern_c::test_write32(1234);
        println!("Readout value is {}", extern_c::test_read32());
    }

    set_leds(0x00);
    let mut mask = 0x40;
    let mut benchmarker: PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> = PrimalSimpleMatch::new();
    loop {
        set_leds(mask);
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
