use heapless::Vec;

cfg_if::cfg_if! {
    if #[cfg(not(feature="wide_index"))] {
        // use 32 bit data types, for less memory usage
        pub type EdgeIndex = u32;
        pub type Timestamp = u32;
        pub type VertexIndex = u32;  // the vertex index in the decoding graph
        pub type NodeIndex = VertexIndex;
        pub type DefectIndex = VertexIndex;
        pub type VertexNodeIndex = VertexIndex;  // must be same as VertexIndex, NodeIndex, DefectIndex
        pub type VertexNum = VertexIndex;
        pub type NodeNum = VertexIndex;
        pub type Weight = i32;
    } else {
        pub type EdgeIndex = usize;
        pub type Timestamp = usize;
        pub type VertexIndex = usize;
        pub type NodeIndex = VertexIndex;
        pub type DefectIndex = VertexIndex;
        pub type VertexNodeIndex = VertexIndex;  // must be same as VertexIndex, NodeIndex, DefectIndex
        pub type VertexNum = VertexIndex;
        pub type NodeNum = VertexIndex;
        pub type Weight = i64;
    }
}

pub const NODE_NONE: NodeIndex = NodeIndex::MAX;

pub struct SlicedVec<'a, T, const N: usize> {
    pub buffer: &'a Vec<T, N>,
    pub start: usize,
    pub end: usize,
}

impl<'a, T, const N: usize> SlicedVec<'a, T, N> {
    pub fn new(buffer: &'a Vec<T, N>, start: usize, end: usize) -> Self {
        debug_assert!(end >= start);
        debug_assert!(end < buffer.len());
        Self { buffer, start, end }
    }
}

#[cfg(any(test, feature = "std"))]
impl<'a, T: std::fmt::Debug, const N: usize> std::fmt::Debug for SlicedVec<'a, T, N> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries((self.start..self.end).map(|index| &self.buffer[index]))
            .finish()
    }
}
