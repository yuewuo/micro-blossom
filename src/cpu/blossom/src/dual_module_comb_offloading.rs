use crate::dual_module_comb::*;
use crate::resources::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::util::*;
use std::cell::{Ref, RefCell};
use std::collections::BTreeSet;

pub struct Offloading {
    /// type information of the offloading
    pub offloading_type: OffloadingType,
    /// affected vertices
    pub affecting_vertices: BTreeSet<VertexIndex>,
    /// affected edges
    pub affecting_edges: BTreeSet<EdgeIndex>,
    /// signals
    pub signals: RefCell<Option<OffloadingSignals>>,
}

pub struct OffloadingSignals {
    /// when this offloading is taking effect
    pub condition: bool,
    /// vertex stalls because of the matching
    pub vertex_stalls: BTreeSet<VertexIndex>,
    /// matched edges
    pub edge_stalls: BTreeSet<EdgeIndex>,
}

impl Offloading {
    pub fn new(offloading_type: OffloadingType, initializer: &SolverInitializer) -> Self {
        let mut affecting_vertices = BTreeSet::new();
        let mut affecting_edges = BTreeSet::new();
        match offloading_type {
            OffloadingType::DefectMatch { edge_index } => {
                affecting_edges.insert(edge_index);
                let (left_index, right_index, _) = initializer.weighted_edges[edge_index];
                affecting_vertices.insert(left_index);
                affecting_vertices.insert(right_index);
            }
            OffloadingType::VirtualMatch {
                edge_index,
                virtual_vertex,
            } => {
                affecting_edges.insert(edge_index);
                let (left_index, right_index, _) = initializer.weighted_edges[edge_index];
                assert!(virtual_vertex == left_index || virtual_vertex == right_index);
                let regular_index = if virtual_vertex == left_index {
                    right_index
                } else {
                    left_index
                };
                affecting_vertices.insert(regular_index);
                for (neighbor_edge_index, &(left_index, right_index, _)) in initializer.weighted_edges.iter().enumerate() {
                    if neighbor_edge_index == edge_index {
                        continue;
                    }
                    if left_index == regular_index {
                        affecting_vertices.insert(right_index);
                    }
                    if right_index == regular_index {
                        affecting_vertices.insert(left_index);
                    }
                }
            }
            OffloadingType::FusionMatch {
                edge_index,
                conditioned_vertex: _,
            } => {
                affecting_edges.insert(edge_index);
                let (left_index, right_index, _) = initializer.weighted_edges[edge_index];
                affecting_vertices.insert(left_index);
                affecting_vertices.insert(right_index);
            }
        }
        Self {
            offloading_type,
            affecting_vertices,
            affecting_edges,
            signals: RefCell::new(None),
        }
    }

    pub fn clear(&mut self) {
        self.register_updated();
    }
    pub fn register_updated(&mut self) {
        self.signals = RefCell::new(None);
    }

    pub fn get_signals(&self, dual_module: &DualModuleCombDriver) -> Ref<'_, OffloadingSignals> {
        referenced_signal!(self.signals, || {
            let mut vertex_stalls = BTreeSet::new();
            let mut edge_stalls = BTreeSet::new();
            let condition = match self.offloading_type {
                OffloadingType::DefectMatch { edge_index } => {
                    let edge = &dual_module.edges[edge_index];
                    let left_vertex = &dual_module.vertices[edge.left_index];
                    let right_vertex = &dual_module.vertices[edge.right_index];
                    let condition = edge.get_post_fetch_is_tight(dual_module)
                        && left_vertex.registers.is_defect
                        && left_vertex.registers.speed == CompactGrowState::Grow
                        && left_vertex.get_is_unique_tight(dual_module)
                        && right_vertex.registers.is_defect
                        && right_vertex.registers.speed == CompactGrowState::Grow
                        && right_vertex.get_is_unique_tight(dual_module);
                    if condition {
                        vertex_stalls.insert(edge.left_index);
                        vertex_stalls.insert(edge.right_index);
                        edge_stalls.insert(edge_index);
                    }
                    condition
                }
                OffloadingType::VirtualMatch {
                    edge_index,
                    virtual_vertex: virtual_index,
                } => {
                    let edge = &dual_module.edges[edge_index];
                    let virtual_vertex = &dual_module.vertices[virtual_index];
                    let regular_index = edge.get_peer(virtual_index);
                    let regular_vertex = &dual_module.vertices[regular_index];
                    let mut condition = edge.get_post_fetch_is_tight(dual_module)
                        && virtual_vertex.registers.is_virtual
                        && regular_vertex.registers.is_defect
                        && regular_vertex.registers.speed == CompactGrowState::Grow;
                    for &neighbor_edge_index in regular_vertex.edge_indices.iter() {
                        if neighbor_edge_index == edge_index {
                            continue;
                        }
                        let neighbor_edge = &dual_module.edges[neighbor_edge_index];
                        let neighbor_vertex_index = neighbor_edge.get_peer(regular_index);
                        let neighbor_vertex = &dual_module.vertices[neighbor_vertex_index];
                        condition &= !neighbor_edge.get_post_fetch_is_tight(dual_module)
                            || (neighbor_vertex.get_is_unique_tight(dual_module) && !neighbor_vertex.registers.is_defect);
                    }
                    if condition {
                        vertex_stalls.insert(regular_index);
                        edge_stalls.insert(edge_index);
                        for &neighbor_edge_index in regular_vertex.edge_indices.iter() {
                            if neighbor_edge_index == edge_index {
                                continue;
                            }
                            let neighbor_edge = &dual_module.edges[neighbor_edge_index];
                            let neighbor_vertex_index = neighbor_edge.get_peer(regular_index);
                            if neighbor_edge.get_post_fetch_is_tight(dual_module) {
                                vertex_stalls.insert(neighbor_vertex_index);
                            }
                        }
                    }
                    condition
                }
                OffloadingType::FusionMatch {
                    edge_index,
                    conditioned_vertex: conditioned_index,
                } => {
                    let edge = &dual_module.edges[edge_index];
                    let conditioned_vertex = &dual_module.vertices[conditioned_index];
                    let regular_index = edge.get_peer(conditioned_index);
                    let regular_vertex = &dual_module.vertices[regular_index];
                    let condition = edge.get_post_fetch_is_tight(dual_module)
                        && conditioned_vertex.registers.is_virtual
                        && regular_vertex.registers.is_defect
                        && regular_vertex.registers.speed == CompactGrowState::Grow
                        && regular_vertex.get_is_isolated(dual_module);
                    if condition {
                        vertex_stalls.insert(regular_index);
                        edge_stalls.insert(edge_index);
                    }
                    condition
                }
            };
            OffloadingSignals {
                condition,
                vertex_stalls,
                edge_stalls,
            }
        })
    }
}
