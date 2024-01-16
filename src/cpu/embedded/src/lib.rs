#![no_std]

pub mod binding;
pub mod util;
pub mod mains {
    automod::dir!(pub "src/mains");
}
#[cfg(feature = "riscv")]
pub mod riscv_driver;

pub use binding::*;
#[cfg(feature = "panic_halt")]
use panic_halt as _;

#[no_mangle]
pub extern "C" fn rust_main_raw() {
    // use EMBEDDED_BLOSSOM_MAIN=<name> to specify main entry
    include!(concat!(env!("OUT_DIR"), "/embedded_blossom_main.name"));
}

pub const RUST_MAIN_NAME: &str = env!("EMBEDDED_BLOSSOM_MAIN_NAME");

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    rust_main_raw();
    println!("[exit]");
    loop {}
}
