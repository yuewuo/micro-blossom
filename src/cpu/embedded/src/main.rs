#![no_std]
#![no_main]

// riscv
#[cfg(feature = "riscv")]
#[no_mangle]
extern "C" fn set_leds(mask: cty::uint32_t) {
    unsafe {
        *(0xF0000000 as *mut u32) = mask;
    }
}
#[cfg(feature = "riscv")]
#[no_mangle]
extern "C" fn print_char(_c: cty::c_char) {
    // TODO
}
#[cfg(feature = "riscv")]
use embedded_blossom::*;
#[cfg(feature = "riscv")]
use riscv_rt::entry;
#[cfg(feature = "riscv")]
#[entry]
fn main() -> ! {
    rust_main();
}
