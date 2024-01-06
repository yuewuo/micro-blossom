pub use core::fmt::Write;

extern "C" {
    pub fn print_char(c: cty::c_char);
    pub fn test_write32(value: cty::c_uint);
    pub fn test_read32() -> cty::c_uint;
}

pub fn print_string(s: &str) {
    for c in s.chars() {
        unsafe { print_char(c as cty::c_char) };
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
