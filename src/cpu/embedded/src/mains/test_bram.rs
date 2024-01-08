use crate::binding::*;

pub fn main() {
    println!("Test BRAM read/write");

    let value = 1234;
    println!("write value: {}", value);
    unsafe {
        extern_c::test_write32(1234);
    }
    println!("read value: {}", unsafe { extern_c::test_read32() });
}
