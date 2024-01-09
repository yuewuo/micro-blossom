use core::arch::asm;
pub use core::fmt::Write;
use core::sync::atomic::{compiler_fence, Ordering};

pub mod extern_c {
    use cty::*;

    extern "C" {
        pub fn print_char(c: c_char);
        pub fn test_write32(value: uint32_t);
        pub fn test_read32() -> uint32_t;
        pub fn set_leds(mask: uint32_t);
        pub fn get_native_time() -> uint64_t;
        pub fn diff_native_time(start: uint64_t, end: uint64_t) -> c_float;
    }
}

pub fn print_string(s: &str) {
    for c in s.chars() {
        unsafe { extern_c::print_char(c as cty::c_char) };
    }
}

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => ({
        let mut printer = Printer;
        writeln!(&mut printer, $($arg)*).unwrap();
    })
}
#[allow(unused_imports)]
pub use println;

pub fn nop_delay(cycles: u32) {
    for _ in 0..cycles {
        unsafe {
            asm!("nop");
        }
    }
}

/// note that it might not be safe to sleep for a long time depending on the C implementation
pub fn sleep(duration: f32) {
    let mut start = unsafe { extern_c::get_native_time() };
    let mut global_diff = 0.;
    // note: this complex implementation is needed because some timer implementation is not capable of
    // recording long time difference, e.g., in Versal board A72 they have 32 bit timer clocked at 150MHz: only capable
    // of recording 28.6s difference. We need to actively accumulating the global timer.
    loop {
        compiler_fence(Ordering::SeqCst);
        let end = unsafe { extern_c::get_native_time() };
        let local_diff = unsafe { extern_c::diff_native_time(start, end) };
        let diff = global_diff + local_diff;
        // avoid overflow by moving the start every 0.5s
        if local_diff > 0.5 {
            start = end;
            global_diff += local_diff;
        }
        if diff >= duration {
            return;
        }
    }
}
