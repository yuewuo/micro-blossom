use core::arch::asm;
pub use core::fmt::Write;

pub mod extern_c {
    use cty::*;

    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct Obstacle {
        pub grown: uint16_t,
        pub conflict: Conflict,
    }

    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct Conflict {
        pub node_1: uint16_t,
        pub node_2: uint16_t,
        pub touch_1: uint16_t,
        pub touch_2: uint16_t,
        pub vertex_1: uint16_t,
        pub vertex_2: uint16_t,
    }

    extern "C" {
        pub fn print_char(c: c_char);
        pub fn test_write32(value: uint32_t);
        pub fn test_read32() -> uint32_t;
        pub fn set_leds(mask: uint32_t);
        pub fn get_native_time() -> uint64_t;
        pub fn diff_native_time(start: uint64_t, end: uint64_t) -> c_float;
        pub fn find_obstacle() -> Obstacle;
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
macro_rules! print {
    ($($arg:tt)*) => ({
        let mut printer = Printer;
        write!(&mut printer, $($arg)*).unwrap();
    })
}
#[allow(unused_imports)]
pub use print;

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
