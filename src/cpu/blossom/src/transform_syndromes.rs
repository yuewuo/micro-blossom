use crate::resources::*;
use clap::Subcommand;
use fusion_blossom::example_codes::*;
use fusion_blossom::mwpm_solver::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Subcommand, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum TransformSyndromesType {
    /// for the rotated surface code from qecp, this will merge the virtual vertices on each side:
    /// those two vertices will have index 0 and 1 respectively.
    QecpRotatedPlanarCode {
        #[clap(value_parser)]
        d: usize,
    },
}

impl TransformSyndromesType {
    pub fn sanity_check(&self, initializer: &SolverInitializer, positions: &Vec<VisualizePosition>) {
        match self {
            Self::QecpRotatedPlanarCode { d } => {
                let d = *d as isize;
                // first verify the graph structure is as expected
                let virtual_vertices: BTreeSet<usize> = initializer.virtual_vertices.iter().cloned().collect();
                let mut max_t = isize::MIN;
                let mut min_t = isize::MAX;
                for (vertex_index, position) in positions.iter().enumerate() {
                    let t = position.t as isize;
                    max_t = std::cmp::max(max_t, t);
                    min_t = std::cmp::min(min_t, t);
                    let i = position.i as isize;
                    let j = position.j as isize;
                    assert_eq!(position.t, t as f64);
                    assert_eq!(position.i, i as f64);
                    assert_eq!(position.j, j as f64);
                    assert_eq!(t % 2, 0);
                    assert_eq!(i % 2, 1);
                    assert_eq!(j % 2, 0);
                    assert!(j - i <= d);
                    assert!(i - j <= d);
                    assert!(i + j >= d);
                    assert!(i + j <= 3 * d);
                    let is_virtual = j - i == d || i - j == d;
                    assert_eq!(virtual_vertices.contains(&vertex_index), is_virtual);
                }
            }
        }
    }

    pub fn run(&self, input_file: String, output_file: String) {
        let mut reader = ErrorPatternReader::new(json!({
            "filename": input_file,
        }));
        let initializer = reader.get_initializer();
        let positions = reader.get_positions();
        match self {
            Self::QecpRotatedPlanarCode { d } => {
                // first verify the graph structure is as expected
                self.sanity_check(&initializer, &positions);
                let d = *d as isize;
                let mut max_t = isize::MIN;
                let mut min_t = isize::MAX;
                for position in positions.iter() {
                    let t = position.t as isize;
                    max_t = std::cmp::max(max_t, t);
                    min_t = std::cmp::min(min_t, t);
                }
                // then collect the two boundaries
                let is_left_boundary = |position: &VisualizePosition| (position.i as isize) - (position.j as isize) == d;
                let is_right_boundary = |position: &VisualizePosition| (position.j as isize) - (position.i as isize) == d;
                let mut new_vertex_indices: Vec<usize> = Vec::with_capacity(initializer.vertex_num);
                // 0 is left boundary, 1 is right boundary
                let mut vertex_num = 2;
                let virtual_t = ((max_t + min_t) / 2) as f64;
                let mut new_positions = vec![
                    VisualizePosition::new(d as f64, 0., virtual_t),
                    VisualizePosition::new(d as f64, (2 * d) as f64, virtual_t),
                ];
                for vertex_index in 0..initializer.vertex_num {
                    if is_left_boundary(&positions[vertex_index]) {
                        new_vertex_indices.push(0);
                    } else if is_right_boundary(&positions[vertex_index]) {
                        new_vertex_indices.push(1);
                    } else {
                        new_vertex_indices.push(vertex_num);
                        new_positions.push(positions[vertex_index].clone());
                        vertex_num += 1;
                    }
                }
                let weighted_edges = initializer
                    .weighted_edges
                    .into_iter()
                    .map(|(a, b, w)| (new_vertex_indices[a], new_vertex_indices[b], w))
                    .collect();
                let new_initializer = SolverInitializer::new(vertex_num, weighted_edges, vec![0, 1]);
                let mut logger = SolverErrorPatternLogger::new(
                    &new_initializer,
                    &new_positions,
                    json!({
                        "filename": output_file,
                    }),
                );
                for _ in 0..reader.syndrome_patterns.len() {
                    let mut syndrome_pattern = reader.generate_random_errors(0);
                    for defect in syndrome_pattern.defect_vertices.iter_mut() {
                        *defect = new_vertex_indices[*defect];
                    }
                    logger.solve_visualizer(&syndrome_pattern, None);
                }
            }
        }
    }

    /// modify the graph
    pub fn parse(&self, mut graph: MicroBlossomSingle) -> MicroBlossomSingle {
        match self {
            Self::QecpRotatedPlanarCode { d } => {
                let initializer = graph.get_initializer();
                let positions = graph.get_positions();
                self.sanity_check(&initializer, &positions);
                let d = *d as isize;
                // first identify the edges that connects the left boundary
                let is_left_boundary = |position: &VisualizePosition| (position.i as isize) - (position.j as isize) == d;
                let mut left_edges = BTreeSet::new();
                for (edge_index, (a, b, _w)) in initializer.weighted_edges.iter().enumerate() {
                    if is_left_boundary(&positions[*a]) || is_left_boundary(&positions[*b]) {
                        left_edges.insert(edge_index);
                    }
                }
                // then find the offloaders associated with the edge
                let mut left_offloaders = vec![];
                for (offloader_index, offloader) in graph.offloading.0.iter().enumerate() {
                    let edge_index = match offloader {
                        OffloadingType::DefectMatch { edge_index } => *edge_index,
                        OffloadingType::VirtualMatch { edge_index, .. } => *edge_index,
                        _ => usize::MAX,
                    };
                    if left_edges.contains(&edge_index) {
                        left_offloaders.push(offloader_index);
                    }
                }
                let mut parity_reporters = ParityReporters::new();
                parity_reporters.add_parity_reporter(left_offloaders);
                graph.parity_reporters = Some(parity_reporters);
                graph
            }
        }
    }
}
