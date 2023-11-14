use crate::dual_module_comb::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use std::cell::{Ref, RefCell};

pub struct Vertex {
    pub vertex_index: VertexIndex,
    pub edge_indices: Vec<EdgeIndex>,
    pub default_is_virtual: bool,
    pub registers: VertexRegisters,
    pub signals: VertexCombSignals,
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
    permit_pre_matching: RefCell<Option<bool>>,
    do_pre_matching: RefCell<Option<bool>>,
    post_execute_signals: RefCell<Option<VertexRegisters>>,
    propagating_peer: RefCell<Option<Option<PropagatingPeer>>>,
    post_update_signals: RefCell<Option<VertexRegisters>>,
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
            permit_pre_matching: RefCell::new(None),
            do_pre_matching: RefCell::new(None),
            post_execute_signals: RefCell::new(None),
            propagating_peer: RefCell::new(None),
            post_update_signals: RefCell::new(None),
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
            default_is_virtual: is_virtual,
            registers: VertexRegisters::new(is_virtual),
            signals: VertexCombSignals::new(),
        }
    }
    pub fn clear(&mut self) {
        self.registers = VertexRegisters::new(self.default_is_virtual);
        self.register_updated();
    }
    pub fn register_updated(&mut self) {
        self.signals = VertexCombSignals::new();
    }

    pub fn get_permit_pre_matching(&self, dual_module: &DualModuleCombDriver) -> bool {
        if !dual_module.use_pre_matching {
            return false;
        }
        self.signals
            .permit_pre_matching
            .borrow_mut()
            .get_or_insert_with(|| {
                self.registers.speed == CompactGrowState::Grow
                    && self
                        .edge_indices
                        .iter()
                        .filter(|&&edge_index| dual_module.edges[edge_index].get_post_fetch_is_tight(dual_module))
                        .count()
                        == 1
            })
            .clone()
    }

    pub fn get_do_pre_matching(&self, dual_module: &DualModuleCombDriver) -> bool {
        if !dual_module.use_pre_matching {
            return false;
        }
        self.signals
            .do_pre_matching
            .borrow_mut()
            .get_or_insert_with(|| {
                self.edge_indices.iter().any(|&edge_index| {
                    let edge = &dual_module.edges[edge_index];
                    edge.get_do_pre_matching(dual_module)
                })
            })
            .clone()
    }

    pub fn get_post_execute_signals(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, VertexRegisters> {
        referenced_signal!(self.signals.post_execute_signals, || {
            let mut signals = self.registers.clone();
            match &dual_module.instruction {
                Instruction::SetSpeed { node, speed } => {
                    if self.registers.node_index == Some(*node) {
                        signals.speed = *speed;
                    }
                }
                Instruction::SetBlossom { node, blossom } => {
                    if self.registers.node_index == Some(*node) || self.registers.root_index == Some(*node) {
                        signals.node_index = Some(*blossom);
                        signals.speed = CompactGrowState::Grow;
                    }
                }
                Instruction::Grow { length } => {
                    if !self.get_do_pre_matching(dual_module) {
                        signals.grown = self.registers.grown + Weight::from(self.registers.speed) * length;
                        assert!(
                            signals.grown >= 0,
                            "vertex {} has negative grown value {}",
                            self.vertex_index,
                            signals.grown
                        );
                    }
                }
                Instruction::AddDefectVertex { vertex, node } => {
                    if self.vertex_index == *vertex {
                        signals.is_defect = true;
                        signals.speed = CompactGrowState::Grow;
                        signals.root_index = Some(*node);
                        signals.node_index = Some(*node);
                    }
                }
                _ => {}
            }
            signals
        })
    }

    pub fn get_propagating_peer(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, Option<PropagatingPeer>> {
        referenced_signal!(self.signals.propagating_peer, || {
            if self.get_post_execute_signals(dual_module).grown == 0 && !self.edge_indices.is_empty() {
                // find a peer node with positive growth and fully-grown edge
                self.edge_indices
                    .iter()
                    .map(|&edge_index| {
                        let edge = &dual_module.edges[edge_index];
                        let peer_index = edge.get_peer(self.vertex_index);
                        let peer = &dual_module.vertices[peer_index];
                        let peer_post_execute_signals = peer.get_post_execute_signals(dual_module);
                        if edge.get_post_execute_is_tight(dual_module)
                            && peer_post_execute_signals.speed == CompactGrowState::Grow
                        {
                            Some(PropagatingPeer {
                                node_index: peer_post_execute_signals.node_index,
                                root_index: peer_post_execute_signals.root_index,
                            })
                        } else {
                            None
                        }
                    })
                    .reduce(|a, b| a.or(b))
                    .unwrap()
            } else {
                None
            }
        })
    }

    pub fn get_post_update_signals(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, VertexRegisters> {
        referenced_signal!(self.signals.post_update_signals, || {
            let mut signals = self.get_post_execute_signals(dual_module).clone();
            let propagating_peer = self.get_propagating_peer(dual_module);
            if !signals.is_defect && !signals.is_virtual && signals.grown == 0 {
                if let Some(peer) = propagating_peer.clone() {
                    signals.node_index = peer.node_index;
                    signals.root_index = peer.root_index;
                    signals.speed = CompactGrowState::Grow;
                } else {
                    signals.node_index = None;
                    signals.root_index = None;
                    signals.speed = CompactGrowState::Stay;
                }
            }
            signals
        })
    }

    pub fn get_shadow_node(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, ShadowNode> {
        referenced_signal!(self.signals.shadow_node, || {
            let signals = self.get_post_execute_signals(dual_module);
            let propagating_peer = self.get_propagating_peer(dual_module);
            let mut shadow_node = ShadowNode {
                node_index: signals.node_index,
                root_index: signals.root_index,
                speed: signals.speed,
            };
            if signals.speed == CompactGrowState::Shrink && signals.grown == 0 {
                if let Some(peer) = propagating_peer.clone() {
                    shadow_node.node_index = peer.node_index;
                    shadow_node.root_index = peer.root_index;
                    shadow_node.speed = CompactGrowState::Grow;
                }
            }
            shadow_node
        })
    }

    /// check for shrinking obstacles
    pub fn get_response(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, CompactObstacle> {
        referenced_signal!(self.signals.response, || {
            if !matches!(dual_module.instruction, Instruction::FindObstacle { .. }) {
                return CompactObstacle::None;
            }
            let post_update_signals = self.get_post_update_signals(dual_module);
            if post_update_signals.speed == CompactGrowState::Shrink {
                return CompactObstacle::GrowLength {
                    length: post_update_signals.grown.try_into().unwrap(),
                };
            }
            CompactObstacle::GrowLength {
                length: CompactWeight::MAX,
            }
        })
    }

    pub fn get_write_signals(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, VertexRegisters> {
        self.get_post_update_signals(dual_module)
    }
}
