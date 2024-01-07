use core::arch::asm;
pub use core::fmt::Write;

pub mod extern_c {
    extern "C" {
        pub fn print_char(c: cty::c_char);
        pub fn test_write32(value: cty::uint32_t);
        pub fn test_read32() -> cty::uint32_t;
        pub fn set_leds(mask: cty::uint32_t);
    }
}

pub fn set_leds(mask: u32) {
    unsafe {
        extern_c::set_leds(mask);
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

fn nop_delay(cycles: u32) {
    for _ in 0..cycles {
        unsafe {
            asm!("nop");
        }
    }
}
