use fusion_blossom::example_codes::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use rand_xoshiro::rand_core::SeedableRng;
use serde_json::json;
use std::collections::HashMap;

/// example code with QEC-Playground as simulator
pub struct QECPlaygroundCode {
    simulator: qecp::simulator::Simulator,
    noise_model: std::sync::Arc<qecp::noise_model::NoiseModel>,
    adaptor: std::sync::Arc<qecp::decoder_fusion::FusionBlossomAdaptor>,
    vertex_index_map: std::sync::Arc<HashMap<usize, VertexIndex>>,
    edge_index_map: std::sync::Arc<HashMap<usize, EdgeIndex>>,
    /// vertices in the code
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for QECPlaygroundCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
    // override simulation function
    #[allow(clippy::unnecessary_cast)]
    fn generate_random_errors(&mut self, seed: u64) -> SyndromePattern {
        use qecp::simulator::SimulatorGenerics;
        let rng = qecp::reproducible_rand::Xoroshiro128StarStar::seed_from_u64(seed);
        self.simulator.set_rng(rng);
        let (error_count, erasure_count) = self.simulator.generate_random_errors(&self.noise_model);
        let sparse_detected_erasures = if erasure_count != 0 {
            self.simulator.generate_sparse_detected_erasures()
        } else {
            qecp::simulator::SparseErasures::new()
        };
        let sparse_measurement = if error_count != 0 {
            self.simulator.generate_sparse_measurement()
        } else {
            qecp::simulator::SparseMeasurement::new()
        };
        let syndrome_pattern = self
            .adaptor
            .generate_syndrome_pattern(&sparse_measurement, &sparse_detected_erasures);
        for vertex in self.vertices.iter_mut() {
            vertex.is_defect = false;
        }
        for &vertex_index in syndrome_pattern.defect_vertices.iter() {
            if let Some(new_index) = self.vertex_index_map.get(&vertex_index) {
                self.vertices[*new_index as usize].is_defect = true;
            }
        }
        for edge in self.edges.iter_mut() {
            edge.is_erasure = false;
        }
        for &edge_index in syndrome_pattern.erasures.iter() {
            if let Some(new_index) = self.edge_index_map.get(&edge_index) {
                self.edges[*new_index as usize].is_erasure = true;
            }
        }
        self.get_syndrome()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QECPlaygroundCodeConfig {
    // default to d
    pub di: Option<usize>,
    pub dj: Option<usize>,
    pub nm: Option<usize>,
    #[serde(default = "qec_playground_default_configs::pe")]
    pub pe: f64,
    pub noise_model_modifier: Option<serde_json::Value>,
    #[serde(default = "qec_playground_default_configs::code_type")]
    pub code_type: qecp::code_builder::CodeType,
    #[serde(default = "qec_playground_default_configs::bias_eta")]
    pub bias_eta: f64,
    pub noise_model: Option<qecp::noise_model_builder::NoiseModelBuilder>,
    #[serde(default = "qec_playground_default_configs::noise_model_configuration")]
    pub noise_model_configuration: serde_json::Value,
    #[serde(default = "qec_playground_default_configs::parallel_init")]
    pub parallel_init: usize,
    #[serde(default = "qec_playground_default_configs::use_brief_edge")]
    pub use_brief_edge: bool,
    // specify the target qubit type
    pub qubit_type: Option<qecp::types::QubitType>,
    #[serde(default = "qecp::decoder_fusion::fusion_default_configs::max_half_weight")]
    pub max_half_weight: usize,
    #[serde(default = "qec_playground_default_configs::trim_isolated_vertices")]
    pub trim_isolated_vertices: bool,
}

pub mod qec_playground_default_configs {
    pub fn pe() -> f64 {
        0.
    }
    pub fn bias_eta() -> f64 {
        0.5
    }
    pub fn noise_model_configuration() -> serde_json::Value {
        json!({})
    }
    pub fn code_type() -> qecp::code_builder::CodeType {
        qecp::code_builder::CodeType::StandardPlanarCode
    }
    pub fn parallel_init() -> usize {
        1
    }
    pub fn use_brief_edge() -> bool {
        false
    }
    pub fn trim_isolated_vertices() -> bool {
        true
    }
}

impl QECPlaygroundCode {
    #[allow(clippy::unnecessary_cast)]
    pub fn new(d: usize, p: f64, config: serde_json::Value) -> Self {
        let config: QECPlaygroundCodeConfig = serde_json::from_value(config).unwrap();
        let di = config.di.unwrap_or(d);
        let dj = config.dj.unwrap_or(d);
        let nm = config.nm.unwrap_or(d);
        let mut simulator = qecp::simulator::Simulator::new(config.code_type, qecp::code_builder::CodeSize::new(nm, di, dj));
        let mut noise_model = qecp::noise_model::NoiseModel::new(&simulator);
        let px = p / (1. + config.bias_eta) / 2.;
        let py = px;
        let pz = p - 2. * px;
        simulator.set_error_rates(&mut noise_model, px, py, pz, config.pe);
        // apply customized noise model
        if let Some(noise_model_builder) = &config.noise_model {
            noise_model_builder.apply(
                &mut simulator,
                &mut noise_model,
                &config.noise_model_configuration,
                p,
                config.bias_eta,
                config.pe,
            );
        }
        simulator.compress_error_rates(&mut noise_model); // by default compress all error rates
        let noise_model = std::sync::Arc::new(noise_model);
        // construct vertices and edges
        let fusion_decoder = qecp::decoder_fusion::FusionDecoder::new(
            &simulator,
            noise_model.clone(),
            &serde_json::from_value(json!({
                "max_half_weight": config.max_half_weight
            }))
            .unwrap(),
            config.parallel_init,
            config.use_brief_edge,
        );
        let adaptor = fusion_decoder.adaptor;
        let initializer = &adaptor.initializer;
        let positions = &adaptor.positions;
        let mut vertex_index_map = HashMap::new();
        // filter the specific qubit type and also remove isolated virtual vertices
        let is_vertex_isolated = if config.trim_isolated_vertices {
            let mut is_vertex_isolated = vec![true; initializer.vertex_num];
            for (left_vertex, right_vertex, _) in initializer.weighted_edges.iter().cloned() {
                is_vertex_isolated[left_vertex] = false;
                is_vertex_isolated[right_vertex] = false;
            }
            is_vertex_isolated
        } else {
            vec![false; initializer.vertex_num]
        };
        for (vertex_index, is_isolated) in is_vertex_isolated.iter().cloned().enumerate() {
            let position = &adaptor.vertex_to_position_mapping[vertex_index];
            let qubit_type = simulator.get_node(position).as_ref().unwrap().qubit_type;
            if !config.qubit_type.is_some_and(|expect| expect != qubit_type) && !is_isolated {
                let new_index = vertex_index_map.len() as VertexIndex;
                vertex_index_map.insert(vertex_index, new_index);
            }
        }
        let mut code = Self {
            simulator,
            noise_model,
            adaptor: adaptor.clone(),
            vertex_index_map: std::sync::Arc::new(vertex_index_map),
            edge_index_map: std::sync::Arc::new(HashMap::new()), // overwrite later
            vertices: Vec::with_capacity(initializer.vertex_num),
            edges: Vec::with_capacity(initializer.weighted_edges.len()),
        };
        let mut edge_index_map = HashMap::new();
        for (edge_index, (left_vertex, right_vertex, weight)) in initializer.weighted_edges.iter().cloned().enumerate() {
            assert!(weight % 2 == 0, "weight must be even number");
            let contains_left = code.vertex_index_map.contains_key(&left_vertex);
            let contains_right = code.vertex_index_map.contains_key(&right_vertex);
            assert_eq!(contains_left, contains_right, "should not connect different type of qubits");
            if contains_left {
                let new_index = edge_index_map.len() as EdgeIndex;
                edge_index_map.insert(edge_index, new_index);
                code.edges.push(CodeEdge {
                    vertices: (code.vertex_index_map[&left_vertex], code.vertex_index_map[&right_vertex]),
                    p: 0.,  // doesn't matter
                    pe: 0., // doesn't matter
                    half_weight: (weight as Weight) / 2,
                    is_erasure: false, // doesn't matter
                });
            }
        }
        code.edge_index_map = std::sync::Arc::new(edge_index_map);
        // automatically create the vertices and nearest-neighbor connection
        code.fill_vertices(code.vertex_index_map.len() as VertexNum);
        // set virtual vertices and positions
        for (vertex_index, position) in positions.iter().cloned().enumerate() {
            if let Some(new_index) = code.vertex_index_map.get(&vertex_index) {
                code.vertices[*new_index as usize].position = VisualizePosition::new(position.i, position.j, position.t);
            }
        }
        for vertex_index in initializer.virtual_vertices.iter() {
            if let Some(new_index) = code.vertex_index_map.get(vertex_index) {
                code.vertices[*new_index as usize].is_virtual = true;
            }
        }
        code
    }
}
