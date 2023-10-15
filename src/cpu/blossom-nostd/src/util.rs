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
