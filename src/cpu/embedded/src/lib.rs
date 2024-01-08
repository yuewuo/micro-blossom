#![no_std]

pub mod binding;
pub mod mains {
    automod::dir!(pub "src/mains");
}
#[cfg(feature = "riscv")]
pub mod riscv_driver;

pub use binding::*;
use panic_halt as _;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // use EMBEDDED_BLOSSOM_MAIN=<name> to specify main entry
    include!(concat!(env!("OUT_DIR"), "/embedded_blossom_main.name"));

    println!("[exit]");
    loop {}
}
