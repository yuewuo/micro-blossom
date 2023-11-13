use crate::dual_module_comb::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::util::*;
use std::cell::RefCell;

pub struct Vertex {
    pub vertex_index: VertexIndex,
    pub edge_indices: Vec<EdgeIndex>,
    pub registers: VertexRegisters,
    pub signals: VertexCombSignals,
}

/// the persistent state of the vertex
pub struct VertexRegisters {
    pub speed: CompactGrowState,
    pub grown: Weight,
    pub is_virtual: bool,
    pub is_defect: bool,
    pub node_index: Option<NodeIndex>,
    pub root_index: Option<NodeIndex>,
}

/// combinatorial signals of the vertex, should be invalidated whenever the registers are updated
pub struct VertexCombSignals {
    permit_pre_matching: RefCell<Option<bool>>,
}

impl VertexRegisters {
    pub fn new() -> Self {
        Self {
            speed: CompactGrowState::Stay,
            grown: 0,
            is_virtual: false,
            is_defect: true,
            node_index: None,
            root_index: None,
        }
    }
}

impl VertexCombSignals {
    pub fn new() -> Self {
        Self {
            permit_pre_matching: RefCell::new(None),
        }
    }
}

impl Vertex {
    pub fn new(vertex_index: VertexIndex, edge_indices: Vec<EdgeIndex>) -> Self {
        Self {
            vertex_index,
            edge_indices,
            registers: VertexRegisters::new(),
            signals: VertexCombSignals::new(),
        }
    }
    pub fn clear(&mut self) {
        self.registers = VertexRegisters::new();
        self.register_updated();
    }
    pub fn register_updated(&mut self) {
        self.signals = VertexCombSignals::new();
    }

    pub fn get_permit_pre_matching(&self, dual_module: &DualModuleCombDriver) -> bool {
        *self.signals.permit_pre_matching.borrow_mut().get_or_insert_with(|| {
            self.registers.speed == CompactGrowState::Grow
                && self
                    .edge_indices
                    .iter()
                    .filter(|&&edge_index| dual_module.edges[edge_index].get_is_tight(dual_module))
                    .count()
                    == 1
        })
    }
}
