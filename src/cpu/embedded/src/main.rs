#![no_std]
#![no_main]

// static mut GLOBAL_STATE: Option<GlobalState> = None;

use core::arch::asm;
use embedded_blossom as _;
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
    let mut mask = 0x40;
    loop {
        set_leds(mask);
        test_acc(mask);
        mask >>= 1;
        if mask == 0 {
            mask = 0x40;
        }
        // delay(300000);
        delay(100); // for simulation
    }
}
