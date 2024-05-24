use crate::mwpm_solver::*;
use crate::resources::*;
use crate::transform_syndromes::*;
use byteorder::{LittleEndian, WriteBytesExt};
use clap::{Args, Parser, Subcommand, ValueEnum};
use fusion_blossom::cli::{ExampleCodeType, RunnableBenchmarkParameters, Verifier};
use fusion_blossom::mwpm_solver::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::VisualizePosition;
use lazy_static::lazy_static;
use serde::Serialize;
use serde_json::json;
use std::convert::AsRef;
use std::env;
use strum_macros::AsRefStr;

cfg_if::cfg_if! {
    if #[cfg(test)] {
        const TEST_EACH_ROUNDS: usize = 20;
    } else {
        const TEST_EACH_ROUNDS: usize = 100;
    }
}

#[derive(Parser, Clone)]
#[clap(author = clap::crate_authors!(", "))]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "Micro Blossom Algorithm for fast Quantum Error Correction Decoding")]
#[clap(color = clap::ColorChoice::Auto)]
#[clap(propagate_version = true)]
#[clap(subcommand_required = true)]
#[clap(arg_required_else_help = true)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// benchmark the speed (and also correctness if enabled)
    Benchmark(BenchmarkParameters),
    Test {
        #[clap(subcommand)]
        command: TestCommands,
    },
    /// parse syndrome file to prepare for Micro Blossom
    Parser(MicroBlossomParserParameters),
    /// transform syndrome file to another syndrome file that is more suitable for hardware implementation
    TransformSyndromes {
        #[clap(value_parser)]
        input_file: String,
        #[clap(value_parser)]
        output_file: String,
        #[clap(subcommand)]
        transform_type: TransformSyndromesType,
    },
}

#[derive(Parser, Clone)]
pub struct BenchmarkParameters {
    /// code distance
    #[clap(value_parser)]
    d: VertexNum,
    /// physical error rate: the probability of each edge to
    #[clap(value_parser)]
    p: f64,
    /// rounds of noisy measurement, valid only when multiple rounds
    #[clap(short = 'e', long, default_value_t = 0.)]
    pe: f64,
    /// rounds of noisy measurement, valid only when multiple rounds
    #[clap(short = 'n', long, default_value_t = 0)]
    noisy_measurements: VertexNum,
    /// maximum half weight of edges
    #[clap(long, default_value_t = 500)]
    max_half_weight: Weight,
    /// example code type
    #[clap(short = 'c', long, value_enum, default_value_t = ExampleCodeType::CodeCapacityPlanarCode)]
    code_type: ExampleCodeType,
    /// the configuration of the code builder
    #[clap(long, default_value_t = ("{}").to_string())]
    code_config: String,
    /// logging to the default visualizer file at visualize/data/visualizer.json
    #[clap(long, action)]
    enable_visualizer: bool,
    /// visualizer file at visualize/data/<visualizer_filename.json>
    #[clap(long, default_value_t = fusion_blossom::visualize::static_visualize_data_filename())]
    pub visualizer_filename: String,
    /// print syndrome patterns
    #[clap(long, action)]
    print_syndrome_pattern: bool,
    /// the method to verify the correctness of the decoding result
    #[clap(long, value_enum, default_value_t = Verifier::FusionSerial)]
    verifier: Verifier,
    /// the number of iterations to run
    #[clap(short = 'r', long, default_value_t = 1000)]
    total_rounds: usize,
    /// select the combination of primal and dual module
    #[clap(short = 'p', long, value_enum, default_value_t = PrimalDualType::EmbeddedComb)]
    primal_dual_type: PrimalDualType,
    /// the configuration of primal and dual module
    #[clap(long, default_value_t = ("{}").to_string())]
    primal_dual_config: String,
    /// message on the progress bar
    #[clap(long, default_value_t = format!(""))]
    pb_message: String,
    /// use deterministic seed for debugging purpose
    #[clap(long, action)]
    use_deterministic_seed: bool,
    /// the benchmark profile output file path
    #[clap(long)]
    benchmark_profiler_output: Option<String>,
    /// skip some iterations, useful when debugging
    #[clap(long, default_value_t = 0)]
    starting_iteration: usize,
    /// when `--primal-dual-type error-pattern-logger`, this option will generate micro blossom configuration {name}.json
    /// and the u32 array binary syndrome defects for embedding into the memory {name}.defects
    #[clap(long, action)]
    parse_micro_blossom_files: bool,
}

#[derive(Parser, Clone)]
pub struct MicroBlossomParserParameters {
    /// syndrome file, could be generated by `--primal-dual-type error-pattern-logger --primal-dual-config '{"filename":...}'`
    #[clap(value_parser)]
    syndromes_file: String,
    /// generate micro blossom graph configuration
    #[clap(long)]
    graph_file: Option<String>,
    /// the u32 array binary syndrome defects for embedding into the memory
    #[clap(long)]
    defects_file: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum PrimalDualType {
    /// embedded primal + standard dual
    PrimalEmbedded,
    /// standard primal + combinatorial dual
    DualComb,
    /// embedded primal + combinatorial dual
    EmbeddedComb,
    /// embedded primal + Scala simulation dual
    EmbeddedScala,
    /// embedded primal + Looper simulated dual
    EmbeddedLooper,
    /// embedded primal + Axi4 simulated dual
    EmbeddedAxi4,
    /// serial primal and dual, standard solution
    Serial,
    /// log error into a file for later fetch
    ErrorPatternLogger,
}

#[derive(Args, Clone)]
pub struct StandardTestParameters {
    /// print out the command to test
    #[clap(short = 'c', long, action)]
    print_command: bool,
    /// enable visualizer
    #[clap(short = 'v', long, action)]
    enable_visualizer: bool,
    /// visualizer file at visualize/data/<visualizer_filename.json>
    #[clap(long, default_value_t = fusion_blossom::visualize::static_visualize_data_filename())]
    pub visualizer_filename: String,
    /// disable the fusion verifier
    #[clap(short = 'd', long, action)]
    disable_fusion: bool,
    /// enable print syndrome pattern
    #[clap(short = 's', long, action)]
    print_syndrome_pattern: bool,
    /// use deterministic seed for debugging purpose
    #[clap(long, action)]
    use_deterministic_seed: bool,
    /// the number of iterations to run
    #[clap(short = 'r', long, default_value_t = TEST_EACH_ROUNDS)]
    total_rounds: usize,
    /// skip some iterations, useful when debugging
    #[clap(long, default_value_t = 0)]
    starting_iteration: usize,
}

#[derive(Subcommand, Clone, AsRefStr)]
enum TestCommands {
    PrimalEmbedded(StandardTestParameters),
    DualComb(StandardTestParameters),
    EmbeddedComb(StandardTestParameters),
    EmbeddedCombPreMatching(StandardTestParameters),
    EmbeddedCombLayerFusion(StandardTestParameters),
    EmbeddedCombPreMatchingLayerFusion(StandardTestParameters),
    EmbeddedScala(StandardTestParameters),
    EmbeddedLooper(StandardTestParameters),
    EmbeddedAxi4(StandardTestParameters),
}

impl From<BenchmarkParameters> for fusion_blossom::cli::BenchmarkParameters {
    fn from(parameters: BenchmarkParameters) -> Self {
        let mut legacy_parameters = fusion_blossom::cli::BenchmarkParameters::parse_from([
            "".to_string(),
            format!("{}", parameters.d),
            format!("{}", parameters.p),
        ]);
        let BenchmarkParameters {
            pe,
            noisy_measurements,
            max_half_weight,
            code_type,
            code_config,
            enable_visualizer,
            visualizer_filename,
            print_syndrome_pattern,
            verifier,
            total_rounds,
            primal_dual_type,
            primal_dual_config,
            pb_message,
            use_deterministic_seed,
            benchmark_profiler_output,
            starting_iteration,
            ..
        } = parameters;
        legacy_parameters.pe = pe;
        legacy_parameters.noisy_measurements = noisy_measurements;
        legacy_parameters.max_half_weight = max_half_weight;
        legacy_parameters.code_type = code_type;
        legacy_parameters.code_config = code_config;
        legacy_parameters.enable_visualizer = enable_visualizer;
        legacy_parameters.visualizer_filename = visualizer_filename;
        legacy_parameters.print_syndrome_pattern = print_syndrome_pattern;
        legacy_parameters.verifier = verifier;
        legacy_parameters.total_rounds = total_rounds;
        match primal_dual_type {
            PrimalDualType::Serial => {
                legacy_parameters.primal_dual_type = fusion_blossom::cli::PrimalDualType::Serial;
                legacy_parameters.primal_dual_config = primal_dual_config;
            }
            PrimalDualType::ErrorPatternLogger => {
                legacy_parameters.primal_dual_type = fusion_blossom::cli::PrimalDualType::ErrorPatternLogger;
                legacy_parameters.primal_dual_config = primal_dual_config;
            }
            _ => {}
        }
        legacy_parameters.pb_message = pb_message;
        legacy_parameters.use_deterministic_seed = use_deterministic_seed;
        legacy_parameters.benchmark_profiler_output = benchmark_profiler_output;
        legacy_parameters.starting_iteration = starting_iteration;
        legacy_parameters
    }
}

impl From<BenchmarkParameters> for RunnableBenchmarkParameters {
    fn from(parameters: BenchmarkParameters) -> Self {
        let mut runnable =
            RunnableBenchmarkParameters::from(fusion_blossom::cli::BenchmarkParameters::from(parameters.clone()));
        // patch the runnable with real primal-dual-solver in this crate
        match parameters.primal_dual_type {
            PrimalDualType::Serial | PrimalDualType::ErrorPatternLogger => {}
            _ => {
                let BenchmarkParameters {
                    code_type,
                    d,
                    p,
                    noisy_measurements,
                    max_half_weight,
                    code_config,
                    primal_dual_type,
                    primal_dual_config,
                    ..
                } = parameters;
                let code_config: serde_json::Value = serde_json::from_str(&code_config).unwrap();
                let primal_dual_config: serde_json::Value = serde_json::from_str(&primal_dual_config).unwrap();
                let code = code_type.build(d, p, noisy_measurements, max_half_weight, code_config);
                let initializer = code.get_initializer();
                let positions = code.get_positions();
                runnable.primal_dual_solver = primal_dual_type.build(&initializer, &positions, primal_dual_config);
            }
        }
        runnable
    }
}

pub fn build_randomized_test_parameters(test_name: String) -> Vec<Vec<String>> {
    let prefix = format!("[{test_name}]");
    let mut parameters = vec![];
    for p in [0.01, 0.03, 0.1, 0.3, 0.499] {
        for d in [3, 7, 11, 15, 19] {
            parameters.push(vec![
                format!("{d}"),
                format!("{p}"),
                format!("--code-type"),
                format!("code-capacity-repetition-code"),
                format!("--pb-message"),
                format!("{prefix} repetition {d} {p}"),
            ]);
        }
    }
    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
        for d in [3, 5, 7, 11, 15] {
            parameters.push(vec![
                format!("{d}"),
                format!("{p}"),
                format!("--code-type"),
                format!("code-capacity-planar-code"),
                format!("--pb-message"),
                format!("{prefix} planar {d} {p}"),
            ]);
        }
    }
    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
        for d in [3, 7, 11] {
            parameters.push(vec![
                format!("{d}"),
                format!("{p}"),
                format!("--code-type"),
                format!("phenomenological-planar-code"),
                format!("--noisy-measurements"),
                format!("{d}"),
                format!("--pb-message"),
                format!("{prefix} phenomenological {d} {p}"),
            ]);
        }
    }
    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
        for d in [3, 7, 11] {
            parameters.push(vec![
                format!("{d}"),
                format!("{p}"),
                format!("--code-type"),
                format!("circuit-level-planar-code"),
                format!("--noisy-measurements"),
                format!("{d}"),
                format!("--pb-message"),
                format!("{prefix} circuit-level {d} {p}"),
            ]);
        }
    }
    parameters
}

impl TestCommands {
    pub fn run(&self) {
        let (primal_dual_type, parameters, primal_dual_config) = match self.clone() {
            TestCommands::PrimalEmbedded(parameters) => ("primal-embedded", parameters, json!({})),
            TestCommands::DualComb(parameters) => ("dual-comb", parameters, json!({})),
            TestCommands::EmbeddedComb(parameters) => ("embedded-comb", parameters, json!({})),
            TestCommands::EmbeddedCombPreMatching(parameters) => (
                "embedded-comb",
                parameters,
                json!({"dual":{"sim_config":{"support_offloading":true}}}),
            ),
            TestCommands::EmbeddedCombLayerFusion(parameters) => (
                "embedded-comb",
                parameters,
                json!({"dual":{"sim_config":{"support_layer_fusion":true}}}),
            ),
            TestCommands::EmbeddedCombPreMatchingLayerFusion(parameters) => (
                "embedded-comb",
                parameters,
                json!({"dual":{"sim_config":{"support_offloading":true,"support_layer_fusion":true}}}),
            ),
            TestCommands::EmbeddedScala(parameters) => ("dual-scala", parameters, json!({})),
            TestCommands::EmbeddedLooper(parameters) => ("dual-looper", parameters, json!({})),
            TestCommands::EmbeddedAxi4(parameters) => ("dual-axi4", parameters, json!({})),
        };
        let command_head = vec![format!(""), format!("benchmark")];
        let mut command_tail = vec!["--total-rounds".to_string(), format!("{}", parameters.total_rounds)];
        if !parameters.disable_fusion {
            command_tail.append(&mut vec![format!("--verifier"), format!("fusion-serial")]);
        } else {
            command_tail.append(&mut vec![format!("--verifier"), format!("none")]);
        }
        if parameters.enable_visualizer {
            command_tail.append(&mut vec![format!("--enable-visualizer")]);
            command_tail.append(&mut vec![format!("--visualizer-filename"), parameters.visualizer_filename]);
        }
        if parameters.print_syndrome_pattern {
            command_tail.append(&mut vec![format!("--print-syndrome-pattern")]);
        }
        if parameters.use_deterministic_seed {
            command_tail.append(&mut vec![format!("--use-deterministic-seed")]);
        }
        command_tail.append(&mut vec![
            format!("--starting-iteration"),
            format!("{}", parameters.starting_iteration),
        ]);
        command_tail.append(&mut vec![format!("--primal-dual-type"), primal_dual_type.to_string()]);
        command_tail.append(&mut vec![format!("--primal-dual-config"), primal_dual_config.to_string()]);
        for parameter in build_randomized_test_parameters(self.as_ref().to_string()).iter() {
            execute_in_cli(
                command_head.iter().chain(parameter.iter()).chain(command_tail.iter()),
                parameters.print_command,
            );
        }
    }
}

impl Cli {
    pub fn run(self) {
        match self.command {
            Commands::Benchmark(benchmark_parameters) => {
                let parse_micro_blossom_files = benchmark_parameters.parse_micro_blossom_files;
                let primal_dual_config = benchmark_parameters.primal_dual_config.clone();
                let runnable = RunnableBenchmarkParameters::from(benchmark_parameters);
                runnable.run();
                if parse_micro_blossom_files {
                    let config: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_str(primal_dual_config.as_str()).unwrap();
                    assert!(
                        config.contains_key("filename"),
                        "filename must be provided in primal-dual-config"
                    );
                    let filename = config.get("filename").unwrap();
                    assert!(filename.is_string(), "filename must be string");
                    let filename = filename.as_str().unwrap();
                    execute_in_cli(
                        [
                            "",
                            "parser",
                            filename,
                            "--graph-file",
                            format!("{filename}.json").as_str(),
                            "--defects-file",
                            format!("{filename}.defects").as_str(),
                        ],
                        true,
                    );
                }
            }
            Commands::Test { command } => command.run(),
            Commands::Parser(parameters) => {
                let code = fusion_blossom::example_codes::ErrorPatternReader::new(json!({
                    "filename": parameters.syndromes_file,
                }));
                // generate graph configuration
                if let Some(graph_file) = parameters.graph_file {
                    let micro_blossom = MicroBlossomSingle::new_code(&code);
                    let json_str = serde_json::to_string(&micro_blossom).unwrap();
                    std::fs::write(graph_file, json_str).unwrap();
                }
                // generate binary file
                if let Some(defects_file) = parameters.defects_file {
                    let mut binary: Vec<u8> = vec![];
                    for syndrome_pattern in code.syndrome_patterns.iter() {
                        for defect in syndrome_pattern.defect_vertices.iter() {
                            let value = *defect as u32;
                            assert_ne!(value, u32::MAX);
                            binary.write_u32::<LittleEndian>(value).unwrap();
                        }
                        binary.write_u32::<LittleEndian>(u32::MAX).unwrap(); // EOF
                    }
                    std::fs::write(defects_file, binary).unwrap();
                }
            }
            Commands::TransformSyndromes {
                transform_type,
                input_file,
                output_file,
            } => transform_type.run(input_file, output_file),
        }
    }
}

pub fn execute_in_cli<I, T>(iter: I, print_command: bool)
where
    I: IntoIterator<Item = T> + Clone,
    T: Into<std::ffi::OsString> + Clone,
{
    if print_command {
        print!("[command]");
        for word in iter.clone() {
            let word = word.clone().into().into_string().unwrap();
            if word.contains(char::is_whitespace) {
                print!("'{word}' ")
            } else {
                print!("{word} ")
            }
        }
        println!();
    }
    Cli::parse_from(iter).run();
}

impl PrimalDualType {
    pub fn build(
        &self,
        initializer: &SolverInitializer,
        positions: &Vec<VisualizePosition>,
        primal_dual_config: serde_json::Value,
    ) -> Box<dyn PrimalDualSolver> {
        // TODO: move this assert to the solvers that actually uses MAX_NODE_NUM
        assert!(
            initializer.vertex_num <= crate::util::MAX_NODE_NUM,
            "potential overflow, increase `MAX_NODE_NUM` when compile the code"
        );
        // TODO: move this stack change to solvers that actually need it
        // stacker::grow(crate::util::MAX_NODE_NUM * 1024, || -> Box<dyn PrimalDualSolver> {
        match self {
            Self::PrimalEmbedded => {
                assert_eq!(primal_dual_config, json!({}));
                Box::new(SolverPrimalEmbedded::new(initializer))
            }
            Self::DualComb => {
                assert_eq!(primal_dual_config, json!({}));
                Box::new(SolverDualComb::new(initializer))
            }
            Self::EmbeddedComb => Box::new(SolverEmbeddedComb::new(initializer)),
            // Self::EmbeddedScala => Box::new(SolverEmbeddedScala::new(initializer)),

            // /// embedded primal + Scala simulation dual
            // EmbeddedScala,
            // /// embedded primal + Looper simulated dual
            // EmbeddedLooper,
            // /// embedded primal + Axi4 simulated dual
            // EmbeddedAxi4,
            // /// serial primal and dual, standard solution
            // Serial,
            // /// log error into a file for later fetch
            // ErrorPatternLogger,
            _ => unimplemented!(),
            // Self::EmbeddedComb => {
            //     let micro_config = MicroBlossomSingle::new(initializer, positions);
            //     Box::new(SolverEmbeddedComb::new_native(micro_config, primal_dual_config))
            // }
            // Self::EmbeddedScala => {
            //     assert_eq!(primal_dual_config, json!({}));
            //     Box::new(SolverEmbeddedScala::new(initializer))
            // }
            // Self::EmbeddedLooper => {
            //     unimplemented!()
            // }
            // Self::EmbeddedAxi4 => {
            //     assert_eq!(primal_dual_config, json!({}));
            //     Box::new(SolverEmbeddedAxi4::new(initializer))
            // }
            // Self::Serial | Self::ErrorPatternLogger => {
            //     unreachable!()
            // }
        }
    }
}
