use crate::binding::*;

pub fn main() {
    println!("Test LED");

    unsafe {
        extern_c::set_leds(0x00);
    }

    let mut mask = 0x40;
    for _ in 0..10 {
        unsafe {
            extern_c::set_leds(mask);
        }
        mask >>= 1;
        if mask == 0 {
            mask = 0x40;
        }
        nop_delay(300000);
        // nop_delay(100); // for faster simulation
    }
}
