//! Register Transfer Level (RTL) Dual Module
//!
//! This is a software implementation of the Micro Blossom algorithm.
//! We assume all the vertices and edges have a single clock source.
//! On every clock cycle, each vertex/edge generates a new vertex/edge as the register state for the next clock cycle.
//! This directly corresponds to an RTL design in HDL language, but without any optimizations.
//! This is supposed to be an algorithm design for Micro Blossom.
//!

use fusion_blossom::dual_module::*;
use fusion_blossom::util::*;

pub struct DualModuleRTL {
    // always reconstruct the whole graph when reset
    pub initializer: SolverInitializer,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub nodes: Vec<DualNodePtr>,
}

pub enum Instruction {
    SetSpeed { node: NodeIndex, speed: DualNodeGrowState },
    SetBlossom { node: NodeIndex, blossom: NodeIndex },
    AddDefectVertex { vertex: VertexIndex },
    FindObstacle { region_preference: usize },
}

pub enum Response {
    NonZeroGrow {
        length: Weight,
    },
    Conflict {
        node_1: NodeIndex,
        node_2: NodeIndex,
        touch_1: NodeIndex,
        touch_2: NodeIndex,
        vertex_1: VertexIndex,
        vertex_2: VertexIndex,
    },
    ConflictVirtual {
        node: NodeIndex,
        touch: NodeIndex,
        vertex: VertexIndex,
        virtual_vertex: VertexIndex,
    },
}

pub fn get_blossom_roots(dual_node_ptr: &DualNodePtr) -> Vec<NodeIndex> {
    let node = dual_node_ptr.read_recursive();
    match &node.class {
        DualNodeClass::Blossom { nodes_circle, .. } => {
            let mut node_indices = vec![];
            for node_ptr in nodes_circle.iter() {
                node_indices.append(&mut get_blossom_roots(&node_ptr.upgrade_force()));
            }
            node_indices
        }
        DualNodeClass::DefectVertex { .. } => vec![node.index],
    }
}

impl DualModuleRTL {
    fn clear(&mut self) {
        // set vertices
        self.vertices = (0..self.initializer.vertex_num)
            .map(|vertex_index| Vertex {
                vertex_index,
                edge_indices: vec![],
                is_virtual: false,
                is_defect: false,
                node_index: None,
                root_index: None,
            })
            .collect();
        // set virtual vertices
        for &virtual_vertex in self.initializer.virtual_vertices.iter() {
            self.vertices[virtual_vertex].is_virtual = true;
        }
        // set edges
        self.edges.clear();
        for (edge_index, &(i, j, weight)) in self.initializer.weighted_edges.iter().enumerate() {
            self.edges.push(Edge {
                edge_index,
                weight,
                left_index: i,
                right_index: j,
                left_growth: 0,
                right_growth: 0,
            });
            for vertex_index in [i, j] {
                self.vertices[vertex_index].edge_indices.push(edge_index);
            }
        }
        // clear nodes
        self.nodes.clear();
    }

    fn new_empty(initializer: &SolverInitializer) -> Self {
        let mut dual_module = DualModuleRTL {
            initializer: initializer.clone(),
            vertices: vec![],
            edges: vec![],
            nodes: vec![],
        };
        dual_module.clear();
        dual_module
    }

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let node = dual_node_ptr.read_recursive();
        assert_eq!(node.index, self.nodes.len());
        self.nodes.push(dual_node_ptr.clone());
        match &node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => {
                // creating blossom is cheap
                for weak_ptr in nodes_circle.iter() {
                    let node_index = weak_ptr.upgrade_force().read_recursive().index;
                    self.execute_instruction(Instruction::SetBlossom {
                        node: node_index,
                        blossom: node.index,
                    });
                }
                // TODO: use priority queue to track shrinking blossom constraint
            }
            DualNodeClass::DefectVertex { defect_index } => {
                self.execute_instruction(Instruction::AddDefectVertex { vertex: *defect_index });
            }
        }
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        // remove blossom is expensive because the vertices doesn't remember all the chain of blossom
        let node = dual_node_ptr.read_recursive();
        let nodes_circle = match &node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => nodes_circle.clone(),
            _ => unreachable!(),
        };
        for weak_ptr in nodes_circle.iter() {
            let node_ptr = weak_ptr.upgrade_force();
            let roots = get_blossom_roots(&node_ptr);
            let blossom_index = node_ptr.read_recursive().index;
            for &root_index in roots.iter() {
                self.execute_instruction(Instruction::SetBlossom {
                    node: root_index,
                    blossom: blossom_index,
                });
            }
        }
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        let node_index = dual_node_ptr.read_recursive().index;
        self.execute_instruction(Instruction::SetSpeed {
            node: node_index,
            speed: grow_state,
        });
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        let return_value = self
            .execute_instruction(Instruction::FindObstacle { region_preference: 0 })
            .unwrap();
        let max_update_length = match return_value {
            Response::NonZeroGrow { length } => MaxUpdateLength::NonZeroGrow((length, false)),
            Response::Conflict {
                node_1,
                node_2,
                touch_1,
                touch_2,
                ..
            } => MaxUpdateLength::Conflicting(
                (self.nodes[node_1].clone(), self.nodes[touch_1].clone()),
                (self.nodes[node_2].clone(), self.nodes[touch_2].clone()),
            ),
            Response::ConflictVirtual {
                node,
                touch,
                virtual_vertex,
                ..
            } => MaxUpdateLength::TouchingVirtual(
                (self.nodes[node].clone(), self.nodes[touch].clone()),
                (virtual_vertex, false),
            ),
            _ => unreachable!(),
        };
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        group_max_update_length.add(max_update_length);
        group_max_update_length
    }

    fn execute_instruction(&mut self, instruction: Instruction) -> Option<Response> {
        None
    }
}

pub struct Vertex {
    pub vertex_index: VertexIndex,
    pub edge_indices: Vec<EdgeIndex>,
    pub is_virtual: bool,
    pub is_defect: bool,
    pub node_index: Option<NodeIndex>, // propagated_dual_node
    pub root_index: Option<NodeIndex>, // propagated_grandson_dual_node
}

pub struct Edge {
    pub edge_index: EdgeIndex,
    pub weight: Weight,
    pub left_index: VertexIndex,
    pub right_index: VertexIndex,
    pub left_growth: Weight,
    pub right_growth: Weight,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dual_module_rtl_vertex_1() {
        // cargo test dual_module_rtl_vertex_1 -- --nocapture
    }
}
