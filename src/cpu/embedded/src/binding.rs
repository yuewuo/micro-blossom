use core::arch::asm;
pub use core::fmt::Write;
pub use micro_blossom_nostd::util::*;

pub mod extern_c {
    use bitflags::bitflags;
    use cty::*;
    use micro_blossom_nostd::interface::*;
    use micro_blossom_nostd::util::*;

    /// SingleReadout allows one to query all information about FindObstacle within single 128 bit read
    #[derive(Debug, Clone, Copy, Default)]
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

    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    pub struct MicroBlossomHardwareInfo {
        pub version: uint32_t,
        pub context_depth: uint32_t,
        pub conflict_channels: uint8_t,
        pub vertex_bits: uint8_t,
        pub weight_bits: uint8_t,
        pub instruction_buffer_depth: uint8_t,
        pub flags: MicroBlossomHardwareFlags,
        pub num_layers: uint8_t,
        pub reserved: uint8_t,
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    pub union MicroBlossomHardwareInfoUnion {
        pub info: MicroBlossomHardwareInfo,
        pub raw: [uint64_t; 2],
    }

    bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(C)]
        pub struct MicroBlossomHardwareFlags: uint16_t {
            const SUPPORT_ADD_DEFECT_VERTEX = 1 << 0;
            const SUPPORT_OFFLOADING = 1 << 1;
            const SUPPORT_LAYER_FUSION = 1 << 2;
            const HARD_CODE_WEIGHTS = 1 << 3;
            const SUPPORT_CONTEXT_SWITCHING = 1 << 4;
            const IS_64_BUS = 1 << 5;
            const SUPPORT_LOAD_STALL_EMULATOR = 1 << 6;
        }
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
        pub fn get_native_frequency() -> c_float;
        pub fn diff_native_time(start: uint64_t, end: uint64_t) -> c_float;
        /// if the time period is small, we can use the cpu counter to quickly obtain the time
        pub fn get_fast_cpu_time() -> uint32_t;
        pub fn get_fast_cpu_duration_ns(start: uint32_t) -> uint32_t;

        pub fn get_hardware_info() -> MicroBlossomHardwareInfo;
        pub fn execute_instruction(instruction: uint32_t, context_id: uint16_t);
        pub fn get_single_readout(context_id: uint16_t) -> SingleReadout;
        pub fn set_maximum_growth(length: uint16_t, context_id: uint16_t);
        pub fn get_maximum_growth(context_id: uint16_t) -> uint16_t;
        pub fn reset_context(context_id: uint16_t);
        pub fn reset_all(context_depth: uint16_t);
        pub fn setup_load_stall_emulator(start_time: uint64_t, interval: uint32_t, context_id: uint16_t);
        pub fn get_last_load_time(context_id: uint16_t) -> uint64_t;
        pub fn get_last_finish_time(context_id: uint16_t) -> uint64_t;

        pub fn clear_instruction_counter();
        pub fn get_instruction_counter() -> uint32_t;
        pub fn clear_readout_counter();
        pub fn get_readout_counter() -> uint32_t;
        pub fn clear_transaction_counter();
        pub fn get_transaction_counter() -> uint32_t;
        pub fn clear_error_counter();
        pub fn get_error_counter() -> uint32_t;
    }

    impl SingleReadout {
        pub fn into_obstacle(self) -> (CompactObstacle, CompactWeight) {
            let grown = self.accumulated_grown as CompactWeight;
            let growable = self.max_growable;
            if growable == u8::MAX {
                (CompactObstacle::None, grown)
            } else if growable != 0 {
                (
                    CompactObstacle::GrowLength {
                        length: growable as CompactWeight,
                    },
                    grown,
                )
            } else if self.conflict_valid != 0 {
                let conflict = CompactObstacle::Conflict {
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
                (conflict, grown)
            } else {
                // when this happens, the DualDriverTracked should check for BlossomNeedExpand event
                // this is usually triggered by reaching maximum growth set by the DualDriverTracked
                (CompactObstacle::GrowLength { length: 0 }, grown)
            }
        }
        pub fn has_conflict(&self) -> bool {
            self.conflict_valid != 0
        }
    }

    impl MicroBlossomHardwareInfo {
        pub unsafe fn reset_all(&self) {
            reset_all(self.context_depth as u16);
        }
    }
}

pub fn print_string(s: &str) {
    for c in s.chars() {
        unsafe { extern_c::print_char(c as cty::c_char) };
    }
}

pub fn nop_delay(cycles: u32) {
    for _ in 0..cycles {
        unsafe {
            asm!("nop");
        }
    }
}
