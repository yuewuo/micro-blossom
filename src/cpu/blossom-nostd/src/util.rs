pub use crate::nonmax;
use num_derive::FromPrimitive;
#[cfg(feature = "serde")]
use serde::*;

cfg_if::cfg_if! {
    if #[cfg(feature="u16_index")] {
        // use 16 bit data types, for less memory usage
        pub type CompactVertexIndex = nonmax::NonMaxU16;
        pub type OptionCompactVertexIndex = nonmax::OptionNonMaxU16;
        pub type CompactVertexNum = u16;
    } else {
        pub type CompactVertexIndex = nonmax::NonMaxU32;
        pub type OptionCompactVertexIndex = nonmax::OptionNonMaxU32;
        pub type CompactVertexNum = u32;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature="u8_layer_id")] {
        pub type CompactLayerNum = u8;
        pub type CompactLayerId = nonmax::NonMaxU8;
        pub type OptionCompactLayerId = nonmax::OptionNonMaxU8;
    } else {
        pub type CompactLayerNum = u32;
        pub type CompactLayerId = nonmax::NonMaxU32;
        pub type OptionCompactLayerId = nonmax::OptionNonMaxU32;
    }
}

pub type CompactNodeIndex = CompactVertexIndex;
pub type CompactDefectIndex = CompactVertexIndex;
pub type CompactVertexNodeIndex = CompactVertexIndex;
pub type OptionCompactNodeIndex = OptionCompactVertexIndex;
pub type OptionCompactDefectIndex = OptionCompactVertexIndex;
pub type OptionCompactVertexNodeIndex = OptionCompactVertexIndex;
pub type CompactNodeNum = CompactVertexNum;

pub type CompactEdgeIndex = u32;
pub type CompactTimestamp = u32;
cfg_if::cfg_if! {
    if #[cfg(feature="i16_weight")] {
        pub type CompactWeight = i16;
    } else {
        pub type CompactWeight = i32;
    }
}

pub type TreeDepth = usize;

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompactGrowState {
    Stay = 0,
    Grow = 1,
    Shrink = 2,
}

impl CompactGrowState {
    pub fn is_conflicting(grow_state_1: CompactGrowState, grow_state_2: CompactGrowState) -> bool {
        match (grow_state_1, grow_state_2) {
            (CompactGrowState::Grow, CompactGrowState::Grow) => true,
            (CompactGrowState::Grow, CompactGrowState::Stay) => true,
            (CompactGrowState::Stay, CompactGrowState::Grow) => true,
            _ => false,
        }
    }
}

impl From<CompactGrowState> for isize {
    fn from(speed: CompactGrowState) -> Self {
        match speed {
            CompactGrowState::Stay => 0,
            CompactGrowState::Shrink => -1,
            CompactGrowState::Grow => 1,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CompactMatchTarget {
    Peer(CompactNodeIndex),
    VirtualVertex(CompactVertexIndex),
}

#[derive(Clone, Copy)]
pub struct TouchingLink {
    /// touching through node index
    pub touch: OptionCompactNodeIndex,
    /// touching through vertex
    pub through: OptionCompactVertexIndex,
    /// peer touches myself through node index
    pub peer_touch: OptionCompactNodeIndex,
    /// peer touches myself through vertex
    pub peer_through: OptionCompactVertexIndex,
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for TouchingLink {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.is_none() {
            formatter.write_str("None")
        } else {
            formatter
                .debug_struct("TouchingLink")
                .field("touch", &self.touch)
                .field("through", &self.through)
                .field("peer_touch", &self.peer_touch)
                .field("peer_through", &self.peer_through)
                .finish()
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature="unsafe_unwrap")] {
        /// unsafe unwrap, only take effect when `unsafe_unwrap` feature is on
        #[macro_export]
        macro_rules! usu {
            ($value:expr) => {
                unsafe { $value.unwrap_unchecked() }
            };
        }

        #[macro_export]
        /// unsafe node index, constructed from any numerical type
        macro_rules! ni {
            ($value:expr) => {
                unsafe { CompactNodeIndex::new_unchecked($value as CompactNodeNum) }
            };
        }
    } else {
        /// safe unwrap
        #[macro_export]
        macro_rules! usu {
            ($value:expr) => {
                $value.unwrap()
            };
        }

        #[macro_export]
        /// node index, constructed from any numerical type
        macro_rules! ni {
            ($value:expr) => {
                CompactNodeIndex::new($value as CompactNodeNum).unwrap()
            };
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature="dangerous_unwrap")] {
        #[macro_export]
        macro_rules! get {
            ($array:expr, $index:expr) => {
                unsafe { ($array.get_unchecked($index)) }
            };
        }
        #[macro_export]
        macro_rules! get_mut {
            ($array:expr, $index:expr) => {
                unsafe { $array.get_unchecked_mut($index) }
            };
        }

        #[macro_export]
        macro_rules! set {
            ($array:expr, $index:expr, $value:expr) => {
                unsafe { *($array.get_unchecked_mut($index)) = $value; }
            };
        }

        #[macro_export]
        macro_rules! unimplemented_or_loop {
            () => {
                loop { }
            };
        }

        #[macro_export]
        macro_rules! unreachable_or_loop {
            () => {
                loop { }
            };
        }
    } else {
        #[macro_export]
        macro_rules! get {
            ($array:expr, $index:expr) => {
                &$array[$index]
            };
        }
        #[macro_export]
        macro_rules! get_mut {
            ($array:expr, $index:expr) => {
                &mut $array[$index]
            };
        }

        #[macro_export]
        macro_rules! set {
            ($array:expr, $index:expr, $value:expr) => {
                $array[$index] = $value;
            };
        }

        #[macro_export]
        macro_rules! unimplemented_or_loop {
            () => {
                unimplemented!()
            };
        }

        #[macro_export]
        macro_rules! unreachable_or_loop {
            () => {
                unreachable!()
            };
        }
    }
}
#[allow(unused_imports)]
pub use get;
#[allow(unused_imports)]
pub use get_mut;
#[allow(unused_imports)]
pub use ni;
#[allow(unused_imports)]
pub use set;
#[allow(unused_imports)]
pub use unimplemented_or_loop;
#[allow(unused_imports)]
pub use unreachable_or_loop;
#[allow(unused_imports)]
pub use usu;

#[cfg(not(feature = "std"))]
pub mod c_printer {
    use core::ffi::c_char;
    pub use core::fmt::Write;

    extern "C" {
        pub fn print_char(c: c_char);
    }

    pub fn print_string(s: &str) {
        for c in s.chars() {
            unsafe { print_char(c as c_char) };
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
}

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
pub use c_printer::print;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
pub use c_printer::println;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
pub use crate::util::c_printer::Printer;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
pub use core::fmt::Write;
