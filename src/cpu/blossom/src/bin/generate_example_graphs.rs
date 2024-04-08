// cargo run --release --bin generate_example_graphs
// see micro-blossom/resources/graphs/README.md

use fusion_blossom::example_codes::*;
use fusion_blossom::visualize::*;
use micro_blossom::resources::*;
use serde_json::json;
use std::env;
use std::fs;

fn generate_example(name: String, code: impl ExampleCode + FusionVisualizer) {
    let folder = "../../../resources/graphs";
    fs::create_dir_all(folder).unwrap();
    let filename = format!("{folder}/example_{name}.json");

    // also generate visualization file if `FUSION_DIR` is set
    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom
    if let Ok(fusion_dir) = env::var("FUSION_DIR") {
        let visualize_filename = format!("micro_blossom_{name}.json");
        let mut visualizer = Visualizer::new(
            Some(format!("{fusion_dir}/visualize/data/{visualize_filename}")),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        visualizer.snapshot("code".to_string(), &code).unwrap();
    }

    println!("generating {name}...");
    let micro_blossom = MicroBlossomSingle::new_code(&code);

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
    for d in [3, 5, 7, 9, 11, 13, 15, 17] {
        generate_example(
            format!("phenomenological_rotated_d{d}"),
            PhenomenologicalRotatedCode::new(d, d, 0.1, max_half_weight),
        );
    }
    for d in [3, 5, 7, 9, 11, 13, 15, 17] {
        let max_half_weight = 7; // do distinguish between different edges
        let config = json!({
            "qubit_type": fusion_blossom::qecp::types::QubitType::StabZ,
            "max_half_weight": max_half_weight,
            "parallel_init": num_cpus::get() - 1,  // speed up construction
        });
        println!("qecp constructing circuit_level_d{d}...");
        generate_example(format!("circuit_level_d{d}"), QECPlaygroundCode::new(d, 0.001, config));
    }
}
