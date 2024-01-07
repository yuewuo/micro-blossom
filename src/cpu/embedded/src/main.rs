#![no_std]
#![no_main]

// riscv
#[cfg(feature = "riscv")]
use embedded_blossom::*;
#[cfg(feature = "riscv")]
use riscv_rt::entry;
#[cfg(feature = "riscv")]
#[entry]
fn main() -> ! {
    rust_main();
}
