// cargo run --bin micro-blossom
// see micro-blossom/resources/graphs/README.md

use fusion_blossom::example_codes::*;
use micro_blossom::resources::*;
use std::fs;

#[allow(clippy::unnecessary_cast)]
fn generate_example(name: &str, code: impl ExampleCode) {
    let folder = "../../../resources/graphs";
    fs::create_dir_all(folder).unwrap();
    let filename = format!("{folder}/example_{name}.json");

    let micro_blossom = MicroBlossomSingle::new_code(code);

    let json_str = serde_json::to_string(&micro_blossom).unwrap();
    fs::write(filename, json_str).unwrap();
}

fn main() {
    let max_half_weight = 1;
    generate_example("code_capacity_d3", CodeCapacityRepetitionCode::new(3, 0.1, max_half_weight));
    generate_example("code_capacity_d5", CodeCapacityRepetitionCode::new(5, 0.1, max_half_weight));
    generate_example(
        "code_capacity_planar_d3",
        CodeCapacityPlanarCode::new(3, 0.1, max_half_weight),
    );
    generate_example(
        "code_capacity_planar_d5",
        CodeCapacityPlanarCode::new(5, 0.1, max_half_weight),
    );
    generate_example(
        "code_capacity_planar_d7",
        CodeCapacityPlanarCode::new(7, 0.1, max_half_weight),
    );
    generate_example(
        "code_capacity_rotated_d3",
        CodeCapacityRotatedCode::new(3, 0.1, max_half_weight),
    );
    generate_example(
        "code_capacity_rotated_d5",
        CodeCapacityRotatedCode::new(5, 0.1, max_half_weight),
    );
    generate_example(
        "code_capacity_rotated_d7",
        CodeCapacityRotatedCode::new(7, 0.1, max_half_weight),
    );
    generate_example(
        "phenomenological_rotated_d3",
        PhenomenologicalRotatedCode::new(3, 3, 0.1, max_half_weight),
    );
    generate_example(
        "phenomenological_rotated_d5",
        PhenomenologicalRotatedCode::new(5, 5, 0.1, max_half_weight),
    );
    generate_example(
        "phenomenological_rotated_d7",
        PhenomenologicalRotatedCode::new(7, 7, 0.1, max_half_weight),
    );
    generate_example(
        "phenomenological_rotated_d9",
        PhenomenologicalRotatedCode::new(9, 9, 0.1, max_half_weight),
    );
    generate_example(
        "phenomenological_rotated_d11",
        PhenomenologicalRotatedCode::new(11, 11, 0.1, 50),
    );
}
