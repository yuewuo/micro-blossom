// cargo run --release --bin generate_example_syndromes
// see micro-blossom/resources/syndromes/README.md

use fusion_blossom::cli::ExampleCodeType;
use fusion_blossom::util::*;
use micro_blossom::cli::execute_in_cli;
use serde_json::json;
use serde_variant::to_variant_name;

const COMMAND_HEAD: &'static [&'static str] = &[
    "",
    "benchmark",
    "--verifier",
    "none",
    "--primal-dual-type",
    "error-pattern-logger",
];

pub fn generate_syndromes(name: String, parameters: &[String]) {
    let folder = "../../../resources/syndromes";
    let primal_dual_config = format!(r#"{{"filename":"{folder}/{name}.syndromes"}}"#);
    execute_in_cli(
        COMMAND_HEAD
            .iter()
            .cloned()
            .chain(["--primal-dual-config", primal_dual_config.as_str()])
            .chain(["--parse-micro-blossom-files"])
            .chain(parameters.iter().map(|x| x.as_str())),
        true,
    );
}

pub fn generate_syndromes_preset(
    name: String,
    d: VertexNum,
    p: f64,
    max_half_weight: Weight,
    code_type: ExampleCodeType,
    parameters: &[String],
) {
    let mut full_parameters: Vec<_> = vec![format!("{d}"), format!("{p}")];
    full_parameters.extend_from_slice(&["--max-half-weight".to_string(), format!("{max_half_weight}")]);
    full_parameters.extend_from_slice(&["--code-type".to_string(), to_variant_name(&code_type).unwrap().to_string()]);
    full_parameters.extend_from_slice(parameters);
    generate_syndromes(name, &full_parameters)
}

fn main() {
    let max_half_weight = 1;
    for d in [3, 5] {
        let p = 0.1;
        generate_syndromes_preset(
            format!("code_capacity_d{d}_p{p}"),
            d,
            p,
            max_half_weight,
            ExampleCodeType::CodeCapacityRepetitionCode,
            &[],
        );
    }
    for d in [3, 5, 7] {
        let p = 0.05;
        generate_syndromes_preset(
            format!("code_capacity_planar_d{d}_p{p}"),
            d,
            p,
            max_half_weight,
            ExampleCodeType::CodeCapacityPlanarCode,
            &[],
        );
    }
    for d in [3, 5, 7] {
        let p = 0.05;
        generate_syndromes_preset(
            format!("code_capacity_rotated_d{d}_p{p}"),
            d,
            p,
            max_half_weight,
            ExampleCodeType::CodeCapacityRotatedCode,
            &[],
        );
    }
    for d in [3, 5, 7, 9, 11, 13, 15, 17] {
        let p = 0.01;
        generate_syndromes_preset(
            format!("phenomenological_rotated_d{d}_p{p}"),
            d,
            p,
            max_half_weight,
            ExampleCodeType::PhenomenologicalRotatedCode,
            &[],
        );
    }
    // the below will need to have `qecp_integrate` feature enabled in `fusion_blossom` package,
    // this will cause cyclic dependency errors. to solve it, one need to pull the fusion blossom package
    // repo locally and change Cargo.toml to use that local clone. Then enable `qecp_integrate`.
    // for d in [3, 5, 7, 9, 11, 13, 15, 17] {
    //     let max_half_weight = 7; // do distinguish between different edges
    //     let p = 0.001;
    //     let config = json!({
    //         "qubit_type": qecp::types::QubitType::StabZ,
    //         "max_half_weight": max_half_weight,
    //         "parallel_init": num_cpus::get() - 1,  // speed up construction
    //     });
    //     println!("qecp constructing circuit_level_d{d}...");
    //     generate_syndromes_preset(
    //         format!("circuit_level_d{d}_p{p}"),
    //         d,
    //         p,
    //         max_half_weight,
    //         ExampleCodeType::QECPlaygroundCode,
    //         &["--code-config".to_string(), serde_json::to_string(&config).unwrap()],
    //     );
    // }
}
