// cargo run --bin micro-blossom
// see micro-blossom/resources/graphs/README.md

use fusion_blossom::example_codes::*;
use micro_blossom::resources::*;
use serde_json::json;
use std::fs;

fn generate_example(name: String, code: impl ExampleCode) {
    let folder = "../../../resources/graphs";
    fs::create_dir_all(folder).unwrap();
    let filename = format!("{folder}/example_{name}.json");

    println!("generating {name}...");
    let micro_blossom = MicroBlossomSingle::new_code(code);

    let json_str = serde_json::to_string(&micro_blossom).unwrap();
    fs::write(filename, json_str).unwrap();
}

fn main() {
    let max_half_weight = 1;
    for d in [3, 5] {
        generate_example(
            format!("code_capacity_d{d}"),
            CodeCapacityRepetitionCode::new(d, 0.1, max_half_weight),
        );
    }
    for d in [3, 5, 7] {
        generate_example(
            format!("code_capacity_planar_d{d}"),
            CodeCapacityPlanarCode::new(d, 0.1, max_half_weight),
        );
    }
    for d in [3, 5, 7] {
        generate_example(
            format!("code_capacity_rotated_d{d}"),
            CodeCapacityRotatedCode::new(d, 0.1, max_half_weight),
        );
    }
    for d in [3, 5, 7, 9, 11] {
        generate_example(
            format!("phenomenological_rotated_d{d}"),
            PhenomenologicalRotatedCode::new(d, d, 0.1, max_half_weight),
        );
    }
    for d in [3, 5, 7, 9, 11] {
        let config = json!({
            "qubit_type": fusion_blossom::qecp::types::QubitType::StabZ,
            "max_half_weight": max_half_weight,
            "parallel_init": num_cpus::get() - 1,  // speed up construction
        });
        println!("qecp constructing circuit_level_d{d}...");
        generate_example(format!("circuit_level_d{d}"), QECPlaygroundCode::new(d, 0.001, config));
    }
}
