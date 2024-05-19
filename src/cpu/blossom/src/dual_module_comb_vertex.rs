use crate::dual_module_comb::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde_json::json;
use std::cell::{Ref, RefCell};

pub struct Vertex {
    pub vertex_index: VertexIndex,
    pub edge_indices: Vec<EdgeIndex>,
    pub default_is_virtual: bool,
    pub offloading_indices: Vec<usize>,
    pub registers: VertexRegisters,
    pub signals: VertexCombSignals,
    /// loading a layer of defects
    pub layer_id: Option<usize>,
}

pub struct VirtualMatchingVertexProfile {
    pub contributing_edges: Vec<EdgeIndex>,
}

/// the persistent state of the vertex
#[derive(Debug, Clone)]
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
    tight_count: RefCell<Option<usize>>,
    offloading_stalled: RefCell<Option<bool>>,
    post_execute_state: RefCell<Option<VertexRegisters>>,
    propagating_peer: RefCell<Option<Option<PropagatingPeer>>>,
    post_update_state: RefCell<Option<VertexRegisters>>,
    shadow_node: RefCell<Option<ShadowNode>>,
    response: RefCell<Option<CompactObstacle>>,
}

#[derive(Debug, Clone)]
pub struct PropagatingPeer {
    pub node_index: Option<NodeIndex>,
    pub root_index: Option<NodeIndex>,
}

#[derive(Debug, Clone)]
pub struct ShadowNode {
    pub speed: CompactGrowState,
    pub node_index: Option<NodeIndex>,
    pub root_index: Option<NodeIndex>,
    pub is_virtual: bool,
}

impl VertexRegisters {
    pub fn new(is_virtual: bool) -> Self {
        Self {
            speed: CompactGrowState::Stay,
            grown: 0,
            is_virtual,
            is_defect: false,
            node_index: if is_virtual { Some(VIRTUAL_NODE_INDEX) } else { None },
            root_index: if is_virtual { Some(VIRTUAL_NODE_INDEX) } else { None },
        }
    }
}

impl VertexCombSignals {
    pub fn new() -> Self {
        Self {
            tight_count: RefCell::new(None),
            offloading_stalled: RefCell::new(None),
            post_execute_state: RefCell::new(None),
            propagating_peer: RefCell::new(None),
            post_update_state: RefCell::new(None),
            shadow_node: RefCell::new(None),
            response: RefCell::new(None),
        }
    }
}

impl Vertex {
    pub fn new(vertex_index: VertexIndex, edge_indices: Vec<EdgeIndex>, is_virtual: bool) -> Self {
        Self {
            vertex_index,
            edge_indices,
            offloading_indices: vec![],
            default_is_virtual: is_virtual,
            registers: VertexRegisters::new(is_virtual),
            signals: VertexCombSignals::new(),
            layer_id: None,
        }
    }
    pub fn clear(&mut self) {
        let is_virtual = if self.layer_id.is_some() {
            true // always start with virtual if waiting for loading syndrome to this layer
        } else {
            self.default_is_virtual
        };
        self.registers = VertexRegisters::new(is_virtual);
        self.register_updated();
    }
    pub fn register_updated(&mut self) {
        self.signals = VertexCombSignals::new();
    }

    pub fn get_tight_count(&self, dual_module: &DualModuleCombDriver) -> usize {
        *referenced_signal!(self.signals.tight_count, || {
            self.edge_indices
                .iter()
                .filter(|&&edge_index| dual_module.edges[edge_index].get_post_fetch_count_tight(dual_module))
                .count()
        })
    }

    pub fn get_is_unique_tight(&self, dual_module: &DualModuleCombDriver) -> bool {
        self.get_tight_count(dual_module) == 1
    }

    pub fn get_is_isolated(&self, dual_module: &DualModuleCombDriver) -> bool {
        self.get_tight_count(dual_module) == 0
    }

    pub fn get_offloading_stalled(&self, dual_module: &DualModuleCombDriver) -> bool {
        referenced_signal!(self.signals.offloading_stalled, || {
            self.offloading_indices
                .iter()
                .map(|&offloading_index| {
                    dual_module.offloading_units[offloading_index]
                        .get_signals(dual_module)
                        .vertex_stalls
                        .contains(&self.vertex_index)
                })
                .reduce(|a, b| a || b)
                .unwrap_or(false)
        })
        .clone()
    }

    pub fn get_post_execute_state(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, VertexRegisters> {
        referenced_signal!(self.signals.post_execute_state, || {
            let mut state = self.registers.clone();
            match &dual_module.instruction {
                Instruction::SetSpeed { node, speed } => {
                    if self.registers.node_index == Some(*node) {
                        state.speed = *speed;
                    }
                }
                Instruction::SetBlossom { node, blossom } => {
                    if self.registers.node_index == Some(*node) || self.registers.root_index == Some(*node) {
                        state.node_index = Some(*blossom);
                        state.speed = CompactGrowState::Grow;
                    }
                }
                Instruction::Grow { length } => {
                    // growth may be disabled if it's pre-matched or it's virtual
                    let mut disable_growth = self.get_offloading_stalled(dual_module);
                    if self.layer_id.is_some() {
                        disable_growth = state.is_virtual;
                    }
                    if !disable_growth {
                        state.grown = self.registers.grown + Weight::from(self.registers.speed) * length;
                        assert!(
                            state.grown >= 0,
                            "vertex {} has negative grown value {}",
                            self.vertex_index,
                            state.grown
                        );
                    }
                }
                Instruction::AddDefectVertex { vertex, node } => {
                    if self.vertex_index == *vertex {
                        state.is_defect = true;
                        state.speed = CompactGrowState::Grow;
                        state.root_index = Some(*node);
                        state.node_index = Some(*node);
                    }
                }
                Instruction::LoadDefectsExternal { time, channel: _ } => {
                    if let Some(layer_id) = self.layer_id {
                        if &layer_id == time {
                            // when loading the layer of syndrome, they are removed from the virtual boundary
                            state.is_virtual = false;
                        }
                    }
                }
                _ => {}
            }
            state
        })
    }

    pub fn get_propagating_peer(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, Option<PropagatingPeer>> {
        referenced_signal!(self.signals.propagating_peer, || {
            if self.get_post_execute_state(dual_module).grown != 0 {
                return None;
            }
            // find a peer node with positive growth and fully-grown edge
            for &edge_index in self.edge_indices.iter() {
                let edge = &dual_module.edges[edge_index];
                let peer_index = edge.get_peer(self.vertex_index);
                let peer = &dual_module.vertices[peer_index];
                let peer_post_execute_state = peer.get_post_execute_state(dual_module);
                if edge.get_post_execute_is_tight(dual_module) && peer_post_execute_state.speed == CompactGrowState::Grow {
                    return Some(PropagatingPeer {
                        node_index: peer_post_execute_state.node_index,
                        root_index: peer_post_execute_state.root_index,
                    });
                }
            }
            None
        })
    }

    pub fn get_post_update_state(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, VertexRegisters> {
        referenced_signal!(self.signals.post_update_state, || {
            let mut state = self.get_post_execute_state(dual_module).clone();
            let propagating_peer = self.get_propagating_peer(dual_module);
            if !state.is_defect && !state.is_virtual && state.grown == 0 {
                if let Some(peer) = propagating_peer.clone() {
                    state.node_index = peer.node_index;
                    state.root_index = peer.root_index;
                    state.speed = CompactGrowState::Grow;
                } else {
                    state.node_index = None;
                    state.root_index = None;
                    state.speed = CompactGrowState::Stay;
                }
            }
            state
        })
    }

    pub fn get_shadow_node(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, ShadowNode> {
        referenced_signal!(self.signals.shadow_node, || {
            let state = self.get_post_update_state(dual_module);
            let propagating_peer = self.get_propagating_peer(dual_module);
            let mut shadow_node = ShadowNode {
                node_index: state.node_index,
                root_index: state.root_index,
                speed: state.speed,
                is_virtual: state.is_virtual,
            };
            if state.speed == CompactGrowState::Shrink && state.grown == 0 {
                if let Some(peer) = propagating_peer.clone() {
                    shadow_node.node_index = peer.node_index;
                    shadow_node.root_index = peer.root_index;
                    shadow_node.speed = CompactGrowState::Grow;
                }
            }
            // compile-time condition
            if self.layer_id.is_some() {
                if state.is_virtual {
                    shadow_node.speed = CompactGrowState::Stay;
                }
            }
            if self.get_offloading_stalled(dual_module) {
                shadow_node.speed = CompactGrowState::Stay;
            }
            shadow_node
        })
    }

    /// check for shrinking obstacles
    pub fn get_response(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, CompactObstacle> {
        referenced_signal!(self.signals.response, || {
            let post_update_state = self.get_post_update_state(dual_module);
            if post_update_state.speed == CompactGrowState::Shrink {
                return CompactObstacle::GrowLength {
                    length: post_update_state.grown.try_into().unwrap(),
                };
            }
            CompactObstacle::GrowLength {
                length: CompactWeight::MAX,
            }
        })
    }

    pub fn get_write_signals(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, VertexRegisters> {
        self.get_post_update_state(dual_module)
    }
}

impl VertexRegisters {
    pub fn snapshot(&self) -> serde_json::Value {
        json!({
            "speed": format!("{:?}", self.speed),
            "grown": self.grown,
            "is_virtual": self.is_virtual,
            "is_defect": self.is_defect,
            "node_index": self.node_index,
            "root_index": self.root_index,
        })
    }
}

impl ShadowNode {
    pub fn snapshot(&self) -> serde_json::Value {
        json!({
            "speed": format!("{:?}", self.speed),
            "node_index": self.node_index,
            "root_index": self.root_index,
        })
    }
}

impl PropagatingPeer {
    pub fn snapshot(&self) -> serde_json::Value {
        json!({
            "node_index": self.node_index,
            "root_index": self.root_index,
        })
    }
}

impl Vertex {
    pub fn snapshot(&self, _abbrev: bool, dual_module: &DualModuleCombDriver) -> serde_json::Value {
        json!({
            "registers": self.registers.snapshot(),
            "signals": json!({
                "offloading_stalled": self.get_offloading_stalled(dual_module),
                "post_execute_state": self.get_post_execute_state(dual_module).snapshot(),
                "propagating_peer": self.get_propagating_peer(dual_module).clone().map(|v| v.snapshot()),
                "post_update_state": self.get_post_update_state(dual_module).snapshot(),
                "shadow_node": self.get_shadow_node(dual_module).snapshot(),
                "response": format!("{:?}", self.get_response(dual_module)),
            })
        })
    }
}
