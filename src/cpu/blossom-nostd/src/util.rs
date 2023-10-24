cfg_if::cfg_if! {
    if #[cfg(feature="u16_index")] {
        // use 16 bit data types, for less memory usage
        pub type CompactVertexIndex = nonmax::NonMaxU16;
        pub type CompactVertexNum = u16;
    } else {
        pub type CompactVertexIndex = nonmax::NonMaxU32;
        pub type CompactVertexNum = u32;
    }
}

pub type CompactNodeIndex = CompactVertexIndex;
pub type CompactDefectIndex = CompactVertexIndex;
pub type CompactVertexNodeIndex = CompactVertexIndex;
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
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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

#[macro_export]
/// node index, constructed from any numerical type
macro_rules! ni {
    ($value:expr) => {
        CompactNodeIndex::new($value as CompactNodeNum).unwrap()
    };
}
#[allow(unused_imports)]
pub use ni;
