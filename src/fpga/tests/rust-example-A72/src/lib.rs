#![no_std]
#![no_main]

use panic_halt as _;

#[no_mangle]
pub extern "C" fn rust_example_add(a: cty::c_int, b: cty::c_int) -> cty::c_int {
    a + b
}
