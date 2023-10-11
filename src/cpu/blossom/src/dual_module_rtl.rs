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
use fusion_blossom::visualize::*;

#[derive(Clone, Debug)]
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
    Grow { length: Weight },
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
    BlossomNeedExpand {
        blossom: NodeIndex,
    },
}

impl Response {
    pub fn reduce(resp1: Option<Response>, resp2: Option<Response>) -> Option<Response> {
        None // TODO
    }
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
    fn execute_instruction(&mut self, instruction: Instruction) -> Option<Response> {
        // register transfer logic
        let vertices_next = self.vertices.iter().map(|vertex| vertex.next(self)).collect();
        let edges_next = self.edges.iter().map(|edge| edge.next(self)).collect();
        let response = self
            .vertices
            .iter()
            .map(|vertex| vertex.respond(self))
            .chain(self.edges.iter().map(|edge| edge.respond(self)))
            .reduce(Response::reduce);
        // update registers
        self.vertices = vertices_next;
        self.edges = edges_next;
        None
    }
}

impl DualModuleImpl for DualModuleRTL {
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

    fn clear(&mut self) {
        // set vertices
        self.vertices = (0..self.initializer.vertex_num)
            .map(|vertex_index| Vertex {
                vertex_index,
                edge_indices: vec![],
                speed: DualNodeGrowState::Stay,
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
        // each vertex must have at least one incident edge
        for vertex in self.vertices.iter() {
            assert!(!vertex.edge_indices.is_empty());
        }
        // clear nodes
        self.nodes.clear();
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
            Response::BlossomNeedExpand { blossom } => MaxUpdateLength::BlossomNeedExpand(self.nodes[blossom].clone()),
            _ => unreachable!(),
        };
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        group_max_update_length.add(max_update_length);
        group_max_update_length
    }

    fn grow(&mut self, length: Weight) {
        self.execute_instruction(Instruction::Grow { length });
    }
}

#[derive(Clone, Debug)]
pub struct Vertex {
    pub vertex_index: VertexIndex,
    pub edge_indices: Vec<EdgeIndex>,
    pub speed: DualNodeGrowState,
    pub is_virtual: bool,
    pub is_defect: bool,
    pub node_index: Option<NodeIndex>, // propagated_dual_node
    pub root_index: Option<NodeIndex>, // propagated_grandson_dual_node
}

impl Vertex {
    // compute the next register values
    fn next(&self, dual_module: &DualModuleRTL) -> Self {
        self.clone()
    }

    // generate a response
    fn respond(&self, dual_module: &DualModuleRTL) -> Option<Response> {
        // only detect when y_S = 0 and delta y_S = -1, whether there are two growing
        if self.speed != DualNodeGrowState::Shrink {
            return None;
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub edge_index: EdgeIndex,
    pub weight: Weight,
    pub left_index: VertexIndex,
    pub right_index: VertexIndex,
    pub left_growth: Weight,
    pub right_growth: Weight,
}

impl Edge {
    // compute the next register values
    fn next(&self, dual_module: &DualModuleRTL) -> Self {
        self.clone()
    }

    // generate a response
    fn respond(&self, dual_module: &DualModuleRTL) -> Option<Response> {
        None
    }
}

impl FusionVisualizer for DualModuleRTL {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let vertices: Vec<serde_json::Value> = self
            .vertices
            .iter()
            .map(|vertex| {
                let mut value = json!({
                    if abbrev { "v" } else { "is_virtual" }: i32::from(vertex.is_virtual),
                    if abbrev { "s" } else { "is_defect" }: i32::from(vertex.is_defect),
                });
                if let Some(node_index) = vertex.node_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "p" } else { "propagated_dual_node" }).to_string(),
                        json!(node_index),
                    );
                }
                if let Some(root_index) = vertex.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "pg" } else { "propagated_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                value
            })
            .collect();
        let edges: Vec<serde_json::Value> = self
            .edges
            .iter()
            .map(|edge| {
                let mut value = json!({
                    if abbrev { "w" } else { "weight" }: edge.weight,
                    if abbrev { "l" } else { "left" }: edge.left_index,
                    if abbrev { "r" } else { "right" }: edge.right_index,
                    if abbrev { "lg" } else { "left_growth" }: edge.left_growth,
                    if abbrev { "rg" } else { "right_growth" }: edge.right_growth,
                });
                let left_vertex = &self.vertices[edge.left_index];
                let right_vertex = &self.vertices[edge.right_index];
                if let Some(node_index) = left_vertex.node_index.as_ref() {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert((if abbrev { "ld" } else { "left_dual_node" }).to_string(), json!(node_index));
                }
                if let Some(root_index) = left_vertex.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "lgd" } else { "left_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                if let Some(node_index) = right_vertex.node_index.as_ref() {
                    value
                        .as_object_mut()
                        .unwrap()
                        .insert((if abbrev { "rd" } else { "right_dual_node" }).to_string(), json!(node_index));
                }
                if let Some(root_index) = right_vertex.root_index.as_ref() {
                    value.as_object_mut().unwrap().insert(
                        (if abbrev { "rgd" } else { "right_grandson_dual_node" }).to_string(),
                        json!(root_index),
                    );
                }
                value
            })
            .collect();
        json!({
            "vertices": vertices,
            "edges": edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusion_blossom::example_codes::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_rtl_basic_1() {
        // cargo test dual_module_rtl_basic_1 -- --nocapture
        let visualize_filename = "dual_module_rtl_basic_1.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);

        let mut visualizer = Visualizer::new(
            option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleRTL::new_empty(&initializer);
        // a simple syndrome
        code.vertices[19].is_defect = true;
        code.vertices[25].is_defect = true;
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer
            .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }
}
