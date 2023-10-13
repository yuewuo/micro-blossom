#![no_std]
#![no_main]

extern crate panic_halt;

use core::arch::asm;
use riscv_rt::entry;

fn delay(cycles: u32) {
    for _ in 0..cycles {
        unsafe {
            asm!("nop");
        }
    }
}

fn set_leds(mask: u32) {
    unsafe {
        *(0x10000000 as *mut u32) = mask;
    }
}

#[entry]
fn main() -> ! {
    let mut mask = 0x40;
    loop {
        set_leds(mask);
        mask >>= 1;
        if mask == 0 {
            mask = 0x40;
        }
        // delay(300000);
        delay(100); // for simulation
    }
}
