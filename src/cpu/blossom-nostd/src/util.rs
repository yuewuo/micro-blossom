cfg_if::cfg_if! {
    if #[cfg(feature="u16_index")] {
        // use 16 bit data types, for less memory usage
        pub type VertexIndex = u16;
    } else {
        pub type VertexIndex = u32;
    }
}

pub type NodeIndex = VertexIndex;
pub type DefectIndex = VertexIndex;
pub type VertexNodeIndex = VertexIndex;
pub type VertexNum = VertexIndex;
pub type NodeNum = VertexIndex;

pub type EdgeIndex = u32;
pub type Timestamp = u32;
pub type Weight = i32; // shouldn't matter

pub const NODE_NONE: NodeIndex = NodeIndex::MAX;

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum GrowState {
    Grow,
    Shrink,
    Stay,
}
