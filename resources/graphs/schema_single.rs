// DO NOT MODIFY MANUALLY!!!
// generate by https://app.quicktype.io/

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::Coordinate;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: Coordinate = serde_json::from_str(&json).unwrap();
// }

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    positions: Vec<Position>,
    vertex_num: i64,
    weighted_edges: Vec<WeightedEdges>,
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
