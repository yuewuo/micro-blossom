use crate::dual_module_comb::*;
use fusion_blossom::util::*;
use std::cell::RefCell;

pub struct Edge {
    pub edge_index: EdgeIndex,
    pub default_weight: Weight,
    pub left_index: VertexIndex,
    pub right_index: VertexIndex,
    pub registers: EdgeRegisters,
    pub signals: EdgeCombSignals,
}

pub struct EdgeRegisters {
    pub weight: Weight,
}

pub struct EdgeCombSignals {
    // left_grown: Weight
    // right_grown: Weight
    is_tight: RefCell<Option<bool>>,
}

impl EdgeRegisters {
    pub fn new(weight: Weight) -> Self {
        Self { weight }
    }
}

impl EdgeCombSignals {
    pub fn new() -> Self {
        Self {
            is_tight: RefCell::new(None),
        }
    }
}

impl Edge {
    pub fn new(edge_index: EdgeIndex, left_index: VertexIndex, right_index: VertexIndex, weight: Weight) -> Self {
        Self {
            edge_index,
            default_weight: weight,
            left_index,
            right_index,
            registers: EdgeRegisters::new(weight),
            signals: EdgeCombSignals::new(),
        }
    }
    pub fn clear(&mut self) {
        self.registers = EdgeRegisters::new(self.default_weight);
        self.register_updated();
    }
    pub fn register_updated(&mut self) {
        self.signals = EdgeCombSignals::new();
    }

    pub fn get_left_grown(&self, dual_module: &DualModuleCombDriver) -> Weight {
        dual_module.vertices[self.left_index].registers.grown
    }

    pub fn get_right_grown(&self, dual_module: &DualModuleCombDriver) -> Weight {
        dual_module.vertices[self.right_index].registers.grown
    }

    pub fn get_is_tight(&self, dual_module: &DualModuleCombDriver) -> bool {
        *self.signals.is_tight.borrow_mut().get_or_insert_with(|| {
            self.get_left_grown(dual_module) + self.get_right_grown(dual_module) >= self.registers.weight
        })
    }
}
