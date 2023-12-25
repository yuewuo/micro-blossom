use crate::dual_module_comb::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde_json::json;
use std::cell::{Ref, RefCell};

pub struct Edge {
    pub edge_index: EdgeIndex,
    pub default_weight: Weight,
    pub left_index: VertexIndex,
    pub right_index: VertexIndex,
    pub offloading_indices: Vec<usize>,
    // each edge can only have one potential virtual matching
    pub potential_virtual_matching: Option<VirtualMatchingEdgeProfile>,
    pub registers: EdgeRegisters,
    pub signals: EdgeCombSignals,
}

pub struct VirtualMatchingEdgeProfile {
    /// the potential virtual vertex: note that a run-time check of the vertex is needed because
    /// the virtual attribute can be removed (although cannot be added)
    pub virtual_index: VertexIndex,
    pub required_untight_edges: Vec<EdgeIndex>,
    pub required_permit_vertices: Vec<VertexIndex>,
}

#[derive(Clone)]
pub struct EdgeRegisters {
    pub weight: Weight,
}

pub struct EdgeCombSignals {
    post_fetch_is_tight: RefCell<Option<bool>>,
    offloading_stalled: RefCell<Option<bool>>,
    post_execute_state: RefCell<Option<EdgeRegisters>>,
    post_execute_is_tight: RefCell<Option<bool>>,
    response: RefCell<Option<CompactObstacle>>,
}

impl EdgeRegisters {
    pub fn new(weight: Weight) -> Self {
        Self { weight }
    }
}

impl EdgeCombSignals {
    pub fn new() -> Self {
        Self {
            post_fetch_is_tight: RefCell::new(None),
            offloading_stalled: RefCell::new(None),
            post_execute_state: RefCell::new(None),
            post_execute_is_tight: RefCell::new(None),
            response: RefCell::new(None),
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
            offloading_indices: vec![],
            potential_virtual_matching: None,
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

    pub fn get_peer(&self, vertex: VertexIndex) -> VertexIndex {
        if vertex == self.left_index {
            self.right_index
        } else if vertex == self.right_index {
            self.left_index
        } else {
            panic!("vertex is not incident to the edge, cannot get peer")
        }
    }

    pub fn get_post_fetch_is_tight(&self, dual_module: &DualModuleCombDriver) -> bool {
        referenced_signal!(self.signals.post_fetch_is_tight, || {
            dual_module.vertices[self.left_index].registers.grown + dual_module.vertices[self.right_index].registers.grown
                >= self.registers.weight
        })
        .clone()
    }

    pub fn get_offloading_stalled(&self, dual_module: &DualModuleCombDriver) -> bool {
        referenced_signal!(self.signals.offloading_stalled, || {
            self.offloading_indices
                .iter()
                .map(|&offloading_index| {
                    dual_module.offloading_units[offloading_index]
                        .get_signals(dual_module)
                        .edge_stalls
                        .contains(&self.edge_index)
                })
                .reduce(|a, b| a || b)
                .unwrap_or(false)
        })
        .clone()
    }

    pub fn get_post_execute_state(&self, _dual_module: &DualModuleCombDriver) -> Ref<'_, EdgeRegisters> {
        referenced_signal!(self.signals.post_execute_state, || {
            let state = self.registers.clone();
            // TODO: dynamically update edge weights
            state
        })
    }

    pub fn get_post_execute_is_tight(&self, dual_module: &DualModuleCombDriver) -> bool {
        referenced_signal!(self.signals.post_execute_is_tight, || {
            let left_vertex = &dual_module.vertices[self.left_index];
            let right_vertex = &dual_module.vertices[self.right_index];
            left_vertex.get_post_execute_state(dual_module).grown + right_vertex.get_post_execute_state(dual_module).grown
                >= self.get_post_execute_state(dual_module).weight
        })
        .clone()
    }

    fn get_remaining(&self, dual_module: &DualModuleCombDriver) -> Weight {
        let left_vertex = dual_module.vertices[self.left_index].get_post_update_state(dual_module);
        let right_vertex = dual_module.vertices[self.right_index].get_post_update_state(dual_module);
        self.get_post_execute_state(dual_module).weight - left_vertex.grown - right_vertex.grown
    }

    pub fn get_response(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, CompactObstacle> {
        referenced_signal!(self.signals.response, || {
            if !matches!(dual_module.instruction, Instruction::FindObstacle { .. }) {
                return CompactObstacle::None;
            }
            let left_shadow = dual_module.vertices[self.left_index].get_shadow_node(dual_module);
            let right_shadow = dual_module.vertices[self.right_index].get_shadow_node(dual_module);
            if left_shadow.node_index == right_shadow.node_index {
                return CompactObstacle::GrowLength {
                    length: CompactWeight::MAX,
                };
            }
            let joint_speed = Weight::from(left_shadow.speed) + Weight::from(right_shadow.speed);
            if joint_speed > 0 {
                let remaining = self.get_remaining(dual_module);
                let node_mapper = |node_index: NodeIndex| -> Option<CompactNodeIndex> {
                    if node_index != VIRTUAL_NODE_INDEX {
                        Some(ni!(node_index))
                    } else {
                        None
                    }
                };
                if remaining == 0 {
                    return CompactObstacle::Conflict {
                        node_1: left_shadow.node_index.and_then(node_mapper),
                        touch_1: left_shadow.root_index.and_then(node_mapper),
                        vertex_1: ni!(self.left_index),
                        node_2: right_shadow.node_index.and_then(node_mapper),
                        touch_2: right_shadow.root_index.and_then(node_mapper),
                        vertex_2: ni!(self.right_index),
                    };
                }
                assert!(
                    remaining % joint_speed == 0,
                    "found a case where the reported maxGrowth is rounding down, edge {}",
                    self.edge_index
                );
                return CompactObstacle::GrowLength {
                    length: (remaining / joint_speed).try_into().unwrap(),
                };
            }
            CompactObstacle::GrowLength {
                length: CompactWeight::MAX,
            }
        })
    }

    pub fn get_write_signals(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, EdgeRegisters> {
        self.get_post_execute_state(dual_module)
    }
}

impl EdgeRegisters {
    pub fn snapshot(&self) -> serde_json::Value {
        json!({
            "weight": self.weight,
        })
    }
}

impl Edge {
    pub fn snapshot(&self, _abbrev: bool, dual_module: &DualModuleCombDriver) -> serde_json::Value {
        json!({
            "registers": json!({
                "weight": self.registers.snapshot(),
            }),
            "signals": json!({
                "post_fetch_is_tight": self.get_post_fetch_is_tight(dual_module),
                "offloading_stalled": self.get_offloading_stalled(dual_module),
                "post_execute_state": self.get_post_execute_state(dual_module).snapshot(),
                "post_execute_is_tight": self.get_post_execute_is_tight(dual_module),
                "response": format!("{:?}", self.get_response(dual_module)),
            })
        })
    }
}
