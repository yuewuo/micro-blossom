// see micro-blossom/resources/graphs/README.md

// generate by https://app.quicktype.io/

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::MicroBlossomSingle;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: MicroBlossomSingle = serde_json::from_str(&json).unwrap();
// }

use fusion_blossom::example_codes::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicroBlossomSingle {
    positions: Vec<Position>,
    vertex_num: i64,
    weighted_edges: Vec<WeightedEdges>,
    virtual_vertices: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    i: f64,
    j: f64,
    t: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedEdges {
    l: i64,
    r: i64,
    w: i64,
}

impl MicroBlossomSingle {
    pub fn new(initializer: &SolverInitializer, positions: &[VisualizePosition]) -> Self {
        Self {
            vertex_num: initializer.vertex_num.try_into().unwrap(),
            positions: positions.iter().map(|p| Position { t: p.t, i: p.i, j: p.j }).collect(),
            weighted_edges: initializer
                .weighted_edges
                .iter()
                .map(|e| WeightedEdges {
                    l: e.0.try_into().unwrap(),
                    r: e.1.try_into().unwrap(),
                    w: e.2 as i64,
                })
                .collect(),
            virtual_vertices: initializer.virtual_vertices.iter().map(|index| *index as i64).collect(),
        }
    }

    pub fn new_code(code: impl ExampleCode) -> Self {
        let initializer = code.get_initializer();
        let positions = code.get_positions();
        assert_eq!(positions.len(), initializer.vertex_num as usize);
        Self::new(&initializer, &positions)
    }

    /// warning: do not use this for production because it doesn't contain useful position information
    /// to ease timing when placing on the hardware; only use this for behavior simulation
    pub fn new_initializer_only(initializer: &SolverInitializer) -> Self {
        let positions: Vec<VisualizePosition> = (0..initializer.vertex_num)
            .map(|_| VisualizePosition::new(0., 0., 0.))
            .collect();
        Self::new(initializer, &positions)
    }
}
