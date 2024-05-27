use core::arch::asm;
pub use core::fmt::Write;

pub mod extern_c {
    use cty::*;
    use micro_blossom_nostd::interface::*;
    use micro_blossom_nostd::util::*;

    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct ReadoutHead {
        /// write to `maximum_growth` will automatically clear `accumulated_grown`
        pub maximum_growth: uint16_t,
        pub accumulated_grown: uint16_t,
        /// usually `growable` and `accumulated_grown` are read simultaneously
        pub growable: uint16_t,
    }

    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct ReadoutConflict {
        pub node_1: uint16_t,
        pub node_2: uint16_t,
        pub touch_1: uint16_t,
        pub touch_2: uint16_t,
        pub vertex_1: uint16_t,
        pub vertex_2: uint16_t,
        pub valid: uint8_t,
    }

    /// SingleReadout allows one to query all information about FindObstacle within single 128 bit read
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct SingleReadout {
        pub node_1: uint16_t,
        pub node_2: uint16_t,
        pub touch_1: uint16_t,
        pub touch_2: uint16_t,
        pub vertex_1: uint16_t,
        pub vertex_2: uint16_t,
        pub conflict_valid: uint8_t,
        pub max_growable: uint8_t,
        pub accumulated_grown: uint16_t,
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub union SingleReadoutUnion {
        pub readout: SingleReadout,
        pub raw: [uint64_t; 2],
    }

    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct MicroBlossomHardwareInfo {
        pub version: uint32_t,
        pub context_depth: uint32_t,
        pub conflict_channels: uint8_t,
        pub vertex_bits: uint8_t,
        pub weight_bits: uint8_t,
        pub instruction_buffer_depth: uint8_t,
        // TODO: add bitflag
    }

    #[derive(Debug, Clone)]
    #[repr(C)]
    pub struct MicroBlossomCounters {
        pub instruction_counter: uint32_t,
        pub readout_counter: uint32_t,
        pub transaction_counter: uint32_t,
        pub error_counter: uint32_t,
    }

    extern "C" {
        pub fn print_char(c: c_char);
        pub fn test_write32(bias: uint32_t, value: uint32_t);
        pub fn test_read32(bias: uint32_t) -> uint32_t;
        pub fn test_write64(bias: uint32_t, value: uint64_t);
        pub fn test_read64(bias: uint32_t) -> uint64_t;
        pub fn test_read128(bias: uint32_t, values: &mut [uint64_t; 2]);
        pub fn test_read256(bias: uint32_t, values: &mut [uint64_t; 4]);
        pub fn set_leds(mask: uint32_t);
        pub fn get_native_time() -> uint64_t;
        pub fn diff_native_time(start: uint64_t, end: uint64_t) -> c_float;

        pub fn get_hardware_info() -> MicroBlossomHardwareInfo;
        pub fn execute_instruction(instruction: uint32_t, context_id: uint16_t);
        pub fn get_conflicts(
            head: *mut ReadoutHead,
            conflicts: *mut ReadoutConflict,
            conflict_channels: uint8_t,
            context_id: uint16_t,
        );
        pub fn set_maximum_growth(length: uint16_t, context_id: uint16_t);

        pub fn clear_instruction_counter();
        pub fn get_instruction_counter() -> uint32_t;
        pub fn clear_readout_counter();
        pub fn get_readout_counter() -> uint32_t;
        pub fn clear_transaction_counter();
        pub fn get_transaction_counter() -> uint32_t;
        pub fn clear_error_counter();
        pub fn get_error_counter() -> uint32_t;
    }

    impl Default for ReadoutConflict {
        fn default() -> Self {
            Self::invalid()
        }
    }

    impl ReadoutConflict {
        pub fn invalid() -> Self {
            Self {
                node_1: 0,
                node_2: 0,
                touch_1: 0,
                touch_2: 0,
                vertex_1: 0,
                vertex_2: 0,
                valid: 0,
            }
        }
        pub fn is_valid(&self) -> bool {
            self.valid != 0
        }
        pub fn get_obstacle(&self) -> CompactObstacle {
            if self.node_1 != u16::MAX {
                return CompactObstacle::Conflict {
                    node_1: ni!(self.node_1).option(),
                    node_2: if self.node_2 == u16::MAX {
                        None.into()
                    } else {
                        ni!(self.node_2).option()
                    },
                    touch_1: ni!(self.touch_1).option(),
                    touch_2: if self.touch_2 == u16::MAX {
                        None.into()
                    } else {
                        ni!(self.touch_2).option()
                    },
                    vertex_1: ni!(self.vertex_1),
                    vertex_2: ni!(self.vertex_2),
                };
            } else {
                return CompactObstacle::Conflict {
                    node_1: ni!(self.node_2).option(),
                    node_2: if self.node_1 == u16::MAX {
                        None.into()
                    } else {
                        ni!(self.node_1).option()
                    },
                    touch_1: ni!(self.touch_2).option(),
                    touch_2: if self.touch_1 == u16::MAX {
                        None.into()
                    } else {
                        ni!(self.touch_1).option()
                    },
                    vertex_1: ni!(self.vertex_2),
                    vertex_2: ni!(self.vertex_1),
                };
            }
        }
    }

    impl ReadoutHead {
        pub const fn new() -> Self {
            Self {
                maximum_growth: 0,
                accumulated_grown: 0,
                growable: 0,
            }
        }
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
        cfg_if::cfg_if! {
            if #[cfg(not(feature="disable_print"))] {
                let mut printer = Printer;
                write!(&mut printer, $($arg)*).unwrap();
            }
        }
    })
}
#[allow(unused_imports)]
pub use print;

#[macro_export]
macro_rules! println {
    () => (print!("\n"));
    ($($arg:tt)*) => ({
        cfg_if::cfg_if! {
            if #[cfg(not(feature="disable_print"))] {
                let mut printer = Printer;
                writeln!(&mut printer, $($arg)*).unwrap();
            }
        }
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
