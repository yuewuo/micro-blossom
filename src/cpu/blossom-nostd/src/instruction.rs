//! Instruction Set Architecture (ISA) of dual accelerator
//!

use crate::util::*;
use num_traits::FromPrimitive;

/// instruction is always 32 bits
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Instruction32(pub u32);

pub const OP_CODE_MASK: u32 = 0b11;
pub const OP_CODE_SET_SPEED: u32 = 0b00;
pub const OP_CODE_SET_BLOSSOM: u32 = 0b01;
pub const OP_CODE_MATCH: u32 = 0b11;
pub const OP_CODE_ADD_DEFECT_VERTEX: u32 = 0b10;

pub const EXTENDED_OP_CODE_ENABLE: u32 = 0b100;
pub const EXTENDED_OP_CODE_MASK: u32 = 0b111 << 3;
pub const EXTENDED_OP_CODE_FIND_OBSTACLE: u32 = 0b000 << 3;
pub const EXTENDED_OP_CODE_CLEAR_ACCUMULATOR: u32 = 0b001 << 3;
pub const EXTENDED_OP_CODE_ACCUMULATE_EDGE: u32 = 0b010 << 3;
pub const EXTENDED_OP_CODE_RESERVED: u32 = 0b011 << 3;
pub const EXTENDED_OP_CODE_RESET: u32 = 0b100 << 3;
pub const EXTENDED_OP_CODE_LOAD_SYNDROME_EXTERNAL: u32 = 0b101 << 3;
pub const EXTENDED_OP_CODE_GROW: u32 = 0b110 << 3;

impl Instruction32 {
    pub fn set_speed(node: CompactNodeIndex, speed: CompactGrowState) -> Self {
        let field_node = (node.get() as u32) << 17;
        let field_speed = (speed as u32) << 15;
        Self(field_node | field_speed | OP_CODE_SET_SPEED)
    }
    pub fn set_blossom(node: CompactNodeIndex, blossom: CompactNodeIndex) -> Self {
        let field_node = (node.get() as u32) << 17;
        let field_blossom = (blossom.get() as u32) << 2;
        Self(field_node | field_blossom | OP_CODE_SET_BLOSSOM)
    }
    pub fn grow(length: CompactWeight) -> Self {
        let field_length = (length as u32) << 6;
        Self(field_length | EXTENDED_OP_CODE_ENABLE | EXTENDED_OP_CODE_GROW)
    }
    pub fn reset() -> Self {
        Self(EXTENDED_OP_CODE_ENABLE | EXTENDED_OP_CODE_RESET)
    }
    pub fn add_defect_vertex(vertex: CompactVertexIndex, node: CompactNodeIndex) -> Self {
        let field_vertex = (vertex.get() as u32) << 17;
        let field_node = (node.get() as u32) << 2;
        Self(field_vertex | field_node | OP_CODE_ADD_DEFECT_VERTEX)
    }
    pub fn reserved() -> Self {
        Self(EXTENDED_OP_CODE_ENABLE | EXTENDED_OP_CODE_RESERVED)
    }
    pub fn find_obstacle() -> Self {
        Self(EXTENDED_OP_CODE_ENABLE | EXTENDED_OP_CODE_FIND_OBSTACLE)
    }

    pub fn is_extended(self) -> bool {
        self.op_code() == OP_CODE_SET_SPEED && (self.0 & EXTENDED_OP_CODE_ENABLE) != 0
    }
    pub fn op_code(self) -> u32 {
        self.0 & OP_CODE_MASK
    }
    pub fn extended_op_code(self) -> u32 {
        self.0 & EXTENDED_OP_CODE_MASK
    }

    pub fn is_set_speed(self) -> bool {
        self.op_code() == OP_CODE_SET_SPEED && (self.0 & EXTENDED_OP_CODE_ENABLE) == 0
    }
    pub fn is_set_blossom(self) -> bool {
        self.op_code() == OP_CODE_SET_BLOSSOM
    }
    pub fn is_match(self) -> bool {
        self.op_code() == OP_CODE_MATCH
    }
    pub fn is_grow(self) -> bool {
        self.is_extended() && self.extended_op_code() == EXTENDED_OP_CODE_GROW
    }

    pub fn field1(self) -> u32 {
        (self.0 >> 17) & ((1 << 15) - 1)
    }
    pub fn get_speed(self) -> CompactGrowState {
        FromPrimitive::from_u32((self.0 >> 15) & ((1 << 2) - 1)).unwrap()
    }

    #[cfg(any(test, feature = "std"))]
    pub fn string_detailed(self) -> String {
        format!("{:?} = {:#08X} = {} = {:#032b}", self, self.0, self.0, self.0)
    }
    #[cfg(any(test, feature = "std"))]
    pub fn print_detailed(self) {
        println!("{}", self.string_detailed());
    }
}

impl Into<u32> for Instruction32 {
    fn into(self) -> u32 {
        self.0
    }
}

#[cfg(any(test, feature = "std"))]
impl std::fmt::Debug for Instruction32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_set_speed() {
            f.debug_struct("SetSpeed")
                .field("node", &self.field1())
                .field("speed", &self.get_speed())
                .finish()
        } else {
            unimplemented!("instruction {:#08X} = {:#032b}", self.0, self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction32_set_speed() {
        // cargo test instruction32_set_speed -- --nocapture
        let instruction = Instruction32::set_speed(ni!(1), CompactGrowState::Grow);
        instruction.print_detailed();
        assert_eq!(format!("{:#032b}", instruction.0), "0b000000000000101000000000000000");
        assert_eq!(
            format!("{:?}", Instruction32::set_speed(ni!(1), CompactGrowState::Grow)),
            "SetSpeed { node: 1, speed: Grow }"
        );
        assert_eq!(
            format!("{:?}", Instruction32::set_speed(ni!(1 << 10), CompactGrowState::Shrink)),
            "SetSpeed { node: 1024, speed: Shrink }"
        );
        assert_eq!(
            format!("{:?}", Instruction32::set_speed(ni!((1 << 15) - 2), CompactGrowState::Stay)),
            "SetSpeed { node: 32766, speed: Stay }"
        );
    }
}
