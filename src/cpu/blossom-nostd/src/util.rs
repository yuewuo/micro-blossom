cfg_if::cfg_if! {
    if #[cfg(feature="u16_index")] {
        // use 16 bit data types, for less memory usage
        pub type VertexIndex = nonmax::NonMaxU16;
        pub type VertexNum = u16;
    } else {
        pub type VertexIndex = nonmax::NonMaxU32;
        pub type VertexNum = u32;
    }
}

pub type NodeIndex = VertexIndex;
pub type DefectIndex = VertexIndex;
pub type VertexNodeIndex = VertexIndex;
pub type NodeNum = VertexNum;

pub type EdgeIndex = u32;
pub type Timestamp = u32;
pub type Weight = i32; // shouldn't matter

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GrowState {
    Grow,
    Shrink,
    Stay,
}

#[macro_export]
macro_rules! ni {
    ($value:expr) => {
        NodeIndex::new($value).unwrap()
    };
}
#[allow(unused_imports)]
pub use ni;
