use konst::{option, primitive::parse_usize, result::unwrap_ctx};

// by default guarantees working at d=15, but can increase if needed
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "3000")));
pub const DOUBLE_MAX_NODE_NUM: usize = MAX_NODE_NUM * 2;

/// a fusion group is a continuous subset of vertices which is recovered simultaneously;
/// it is required that
pub struct FusionGroups {}
