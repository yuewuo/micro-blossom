use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use num_derive::FromPrimitive;

pub struct RiscVCommandDriver {
    pub base_register: usize,
}

impl DualStacklessDriver for RiscVCommandDriver {
    fn clear(&mut self) {
        unimplemented!()
    }
    fn set_speed(&mut self, node: NodeIndex, speed: GrowState) {
        self.write_argument::<1>(node.into());
        self.write_argument::<2>(speed as u32);
        self.write_opcode(OpCode::SetSpeed);
    }
    fn set_blossom(&mut self, node: NodeIndex, blossom: NodeIndex) {
        self.write_argument::<1>(node.into());
        self.write_argument::<2>(blossom.into());
        self.write_opcode(OpCode::SetBlossom);
    }
    fn find_obstacle(&mut self) -> MaxUpdateLength {
        self.write_opcode(OpCode::FindObstacle);
        let rspcode = self.read_rspcode();
        match rspcode {
            RspCode::NonZeroGrow => MaxUpdateLength::GrowLength {
                length: self.read_argument::<5>() as Weight,
            },
            RspCode::Conflict => MaxUpdateLength::Conflict {
                node_1: self.read_argument::<5>() as NodeIndex,
                node_2: self.read_argument::<6>() as NodeIndex,
                touch_1: self.read_argument::<7>() as NodeIndex,
                touch_2: self.read_argument::<8>() as NodeIndex,
                vertex_1: self.read_argument::<9>() as VertexIndex,
                vertex_2: self.read_argument::<10>() as VertexIndex,
            },
            RspCode::BlossomNeedExpand => MaxUpdateLength::BlossomNeedExpand {
                blossom: self.read_argument::<5>() as NodeIndex,
            },
        }
    }
    fn grow(&mut self, length: Weight) {
        self.write_argument::<1>(length as u32);
        self.write_opcode(OpCode::Grow);
    }
}

// 4 write registers and 8 read registers
const REGISTER_INTERVAL: usize = 0x0010; // at most 1024 virtual devices supported

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
pub enum OpCode {
    SetSpeed,
    SetBlossom,
    Match,
    Grow,
    FindObstacle,
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy, Debug, FromPrimitive)]
pub enum RspCode {
    NonZeroGrow,
    Conflict,
    BlossomNeedExpand,
}

impl RiscVCommandDriver {
    pub fn new(base_register: usize) -> Self {
        Self { base_register }
    }

    fn write_opcode(&self, opcode: OpCode) {
        self.write_argument::<0>(opcode as u32);
    }

    fn write_argument<const INDEX: usize>(&self, argument: u32) {
        unsafe {
            *((self.base_register + INDEX * REGISTER_INTERVAL) as *mut u32) = argument;
        }
    }

    fn read_rspcode(&self) -> RspCode {
        num::FromPrimitive::from_u32(self.read_argument::<4>()).unwrap()
    }

    fn read_argument<const INDEX: usize>(&self) -> u32 {
        unsafe { *((self.base_register + INDEX * REGISTER_INTERVAL) as *mut u32) }
    }
}
