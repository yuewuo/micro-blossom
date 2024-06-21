// cargo run --release --bin paper_figures
// to render the images, use paper_micro_blossom_fusion_demo.vue
//    in https://github.com/yuewuo/conference-talk-2023-APS-march-meeting
//    that is, run

use fusion_blossom::dual_module::*;
use fusion_blossom::dual_module_parallel::*;
use fusion_blossom::dual_module_serial::*;
use fusion_blossom::example_codes::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::primal_module_parallel::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use serde_json::json;

fn main() {
    micro_fusion_demo();
    micro_paper_code_capacity_demo();
    micro_paper_circuit_level_demo();
    micro_paper_phenomenological_demo();
}

const MICRO_FUSION_DEMO_RNG_SEED: u64 = 671;

fn micro_fusion_demo() {
    let visualize_filename = "micro_fusion_demo.json".to_string();
    let d = 5;
    let mut code = QECPlaygroundCode::new(
        d,
        0.01,
        json!({
            "nm": d - 1,
            "code_type": fusion_blossom::qecp::code_builder::CodeType::RotatedPlanarCode,
            "noise_model": fusion_blossom::qecp::noise_model_builder::NoiseModelBuilder::StimNoiseModel,
            "qubit_type": fusion_blossom::qecp::types::QubitType::StabZ,
            "max_half_weight": 7,
            "trim_isolated_vertices": false,
        }),
    );
    let random_syndrome = code.generate_random_errors(MICRO_FUSION_DEMO_RNG_SEED);
    let defect_vertices = random_syndrome.defect_vertices;
    let layer = (d + 1) * (d + 1) / 2;
    let mut partition_config = PartitionConfig::new(layer * d);
    partition_config.partitions.clear();
    let mut last_id = 0;
    for idx in 0..d + 1 {
        partition_config.partitions.push(VertexRange::new(layer * idx, layer * idx));
        if idx >= 1 {
            partition_config.fusions.push((last_id, idx));
            last_id = partition_config.fusions.len() + d;
        }
    }

    println!("partition_config: {partition_config:?}");
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    let initializer = code.get_initializer();
    let partition_info = partition_config.info();
    // create dual module
    let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(
        &initializer,
        &partition_info,
        DualModuleParallelConfig::default(),
    );
    let primal_config = PrimalModuleParallelConfig {
        debug_sequential: true,
        ..Default::default()
    };
    let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
    code.set_defect_vertices(&defect_vertices);
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &dual_module, Some(&mut visualizer));
    let useless_interface_ptr = DualModuleInterfacePtr::new_empty(); // don't actually use it
    let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    let last_interface_ptr = &primal_module.units.last().unwrap().read_recursive().interface_ptr;
    visualizer
        .snapshot_combined(
            "perfect matching and subgraph".to_string(),
            vec![
                last_interface_ptr,
                &dual_module,
                &perfect_matching,
                &VisualizeSubgraph::new(&subgraph),
            ],
        )
        .unwrap();
}

fn micro_paper_code_capacity_demo() {
    let visualize_filename = "micro_paper_code_capacity_demo.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(vec![7, 10]);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    let subgraph = vec![12];
    visualizer
        .snapshot_combined(
            "syndrome".to_string(),
            vec![&interface_ptr, &dual_module, &VisualizeSubgraph::new(&subgraph)],
        )
        .unwrap();
}

fn micro_paper_circuit_level_demo() {
    let visualize_filename = "micro_paper_circuit_level_demo.json".to_string();
    let d = 5;
    let code = QECPlaygroundCode::new(
        d,
        0.001,
        json!({
            "nm": d - 1,
            "code_type": fusion_blossom::qecp::code_builder::CodeType::RotatedPlanarCode,
            "noise_model": fusion_blossom::qecp::noise_model_builder::NoiseModelBuilder::StimNoiseModel,
            "qubit_type": fusion_blossom::qecp::types::QubitType::StabZ,
            "max_half_weight": 7,
            "trim_isolated_vertices": false,
        }),
    );
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(vec![45, 68]);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    let subgraph = vec![149];
    visualizer
        .snapshot_combined(
            "syndrome".to_string(),
            vec![&interface_ptr, &dual_module, &VisualizeSubgraph::new(&subgraph)],
        )
        .unwrap();
}

fn micro_paper_phenomenological_demo() {
    let visualize_filename = "micro_paper_phenomenological_demo.json".to_string();
    let d = 5;
    let code = QECPlaygroundCode::new(
        d,
        0.001,
        json!({
            "nm": d - 1,
            "code_type": fusion_blossom::qecp::code_builder::CodeType::RotatedPlanarCode,
            "noise_model": fusion_blossom::qecp::noise_model_builder::NoiseModelBuilder::Phenomenological,
            "qubit_type": fusion_blossom::qecp::types::QubitType::StabZ,
            "max_half_weight": 7,
            "trim_isolated_vertices": false,
        }),
    );
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(vec![45, 68]);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    let subgraph = vec![85, 95];
    visualizer
        .snapshot_combined(
            "syndrome".to_string(),
            vec![&interface_ptr, &dual_module, &VisualizeSubgraph::new(&subgraph)],
        )
        .unwrap();
}
