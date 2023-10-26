use crate::mwpm_solver::*;
use clap::{Parser, Subcommand, ValueEnum};
use fusion_blossom::cli::{ExampleCodeType, RunnableBenchmarkParameters, Verifier};
use fusion_blossom::mwpm_solver::*;
use fusion_blossom::util::*;
use serde::Serialize;
use serde_json::json;
use std::env;

const TEST_EACH_ROUNDS: usize = 100;

#[derive(Parser, Clone)]
#[clap(author = clap::crate_authors!(", "))]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "Fusion Blossom Algorithm for fast Quantum Error Correction Decoding")]
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
    #[clap(short = 'p', long, value_enum, default_value_t = PrimalDualType::DualRTL)]
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
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum PrimalDualType {
    /// standard primal + RTL-behavior dual
    DualRTL,
    /// embedded primal + standard dual
    PrimalEmbedded,
    /// serial primal and dual, standard solution
    Serial,
    /// log error into a file for later fetch
    ErrorPatternLogger,
}

#[derive(Subcommand, Clone)]
enum TestCommands {
    /// test RTL-behavior dual module
    DualRTL {
        /// print out the command to test
        #[clap(short = 'c', long, action)]
        print_command: bool,
        /// enable visualizer
        #[clap(short = 'v', long, action)]
        enable_visualizer: bool,
        /// disable the fusion verifier
        #[clap(short = 'd', long, action)]
        disable_fusion: bool,
        /// enable print syndrome pattern
        #[clap(short = 's', long, action)]
        print_syndrome_pattern: bool,
    },
    /// test embedded primal module
    PrimalEmbedded {
        /// print out the command to test
        #[clap(short = 'c', long, action)]
        print_command: bool,
        /// enable visualizer
        #[clap(short = 'v', long, action)]
        enable_visualizer: bool,
        /// disable the fusion verifier
        #[clap(short = 'd', long, action)]
        disable_fusion: bool,
        /// enable print syndrome pattern
        #[clap(short = 's', long, action)]
        print_syndrome_pattern: bool,
    },
}

impl From<BenchmarkParameters> for fusion_blossom::cli::BenchmarkParameters {
    fn from(parameters: BenchmarkParameters) -> Self {
        let mut legacy_parameters = fusion_blossom::cli::BenchmarkParameters::parse_from([
            "".to_string(),
            format!("{}", parameters.d),
            format!("{}", parameters.p),
        ]);
        let BenchmarkParameters {
            d: _,
            p: _,
            pe,
            noisy_measurements,
            max_half_weight,
            code_type,
            code_config,
            enable_visualizer,
            print_syndrome_pattern,
            verifier,
            total_rounds,
            primal_dual_type,
            primal_dual_config,
            pb_message,
            use_deterministic_seed,
            benchmark_profiler_output,
            starting_iteration,
        } = parameters;
        legacy_parameters.pe = pe;
        legacy_parameters.noisy_measurements = noisy_measurements;
        legacy_parameters.max_half_weight = max_half_weight;
        legacy_parameters.code_type = code_type;
        legacy_parameters.code_config = code_config;
        legacy_parameters.enable_visualizer = enable_visualizer;
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
                runnable.primal_dual_solver = primal_dual_type.build(&initializer, primal_dual_config);
            }
        }
        runnable
    }
}

impl Cli {
    pub fn run(self) {
        match self.command {
            Commands::Benchmark(benchmark_parameters) => {
                let runnable = RunnableBenchmarkParameters::from(benchmark_parameters);
                runnable.run();
            }
            Commands::Test { command } => match command {
                TestCommands::DualRTL {
                    print_command,
                    enable_visualizer,
                    disable_fusion,
                    print_syndrome_pattern,
                } => {
                    let mut parameters = vec![];
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11, 15, 19] {
                            parameters.push(vec![
                                format!("{d}"),
                                format!("{p}"),
                                format!("--code-type"),
                                format!("code-capacity-repetition-code"),
                                format!("--pb-message"),
                                format!("repetition {d} {p}"),
                            ]);
                        }
                    }
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11, 15, 19] {
                            parameters.push(vec![
                                format!("{d}"),
                                format!("{p}"),
                                format!("--code-type"),
                                format!("code-capacity-planar-code"),
                                format!("--pb-message"),
                                format!("planar {d} {p}"),
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
                                format!("phenomenological {d} {p}"),
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
                                format!("circuit-level {d} {p}"),
                            ]);
                        }
                    }
                    let command_head = vec![format!(""), format!("benchmark")];
                    let mut command_tail = vec!["--total-rounds".to_string(), format!("{TEST_EACH_ROUNDS}")];
                    if !disable_fusion {
                        command_tail.append(&mut vec![format!("--verifier"), format!("fusion-serial")]);
                    } else {
                        command_tail.append(&mut vec![format!("--verifier"), format!("none")]);
                    }
                    if enable_visualizer {
                        command_tail.append(&mut vec![format!("--enable-visualizer")]);
                    }
                    if print_syndrome_pattern {
                        command_tail.append(&mut vec![format!("--print-syndrome-pattern")]);
                    }
                    command_tail.append(&mut vec![format!("--primal-dual-type"), format!("dual-rtl")]);
                    for parameter in parameters.iter() {
                        execute_in_cli(
                            command_head.iter().chain(parameter.iter()).chain(command_tail.iter()),
                            print_command,
                        );
                    }
                }
                TestCommands::PrimalEmbedded {
                    print_command,
                    enable_visualizer,
                    disable_fusion,
                    print_syndrome_pattern,
                } => {
                    let mut parameters = vec![];
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11, 15, 19] {
                            parameters.push(vec![
                                format!("{d}"),
                                format!("{p}"),
                                format!("--code-type"),
                                format!("code-capacity-repetition-code"),
                                format!("--pb-message"),
                                format!("repetition {d} {p}"),
                            ]);
                        }
                    }
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11, 15, 19] {
                            parameters.push(vec![
                                format!("{d}"),
                                format!("{p}"),
                                format!("--code-type"),
                                format!("code-capacity-planar-code"),
                                format!("--pb-message"),
                                format!("planar {d} {p}"),
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
                                format!("phenomenological {d} {p}"),
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
                                format!("circuit-level {d} {p}"),
                            ]);
                        }
                    }
                    let command_head = vec![format!(""), format!("benchmark")];
                    let mut command_tail = vec!["--total-rounds".to_string(), format!("{TEST_EACH_ROUNDS}")];
                    if !disable_fusion {
                        command_tail.append(&mut vec![format!("--verifier"), format!("fusion-serial")]);
                    } else {
                        command_tail.append(&mut vec![format!("--verifier"), format!("none")]);
                    }
                    if enable_visualizer {
                        command_tail.append(&mut vec![format!("--enable-visualizer")]);
                    }
                    if print_syndrome_pattern {
                        command_tail.append(&mut vec![format!("--print-syndrome-pattern")]);
                    }
                    command_tail.append(&mut vec![format!("--primal-dual-type"), format!("primal-embedded")]);
                    for parameter in parameters.iter() {
                        execute_in_cli(
                            command_head.iter().chain(parameter.iter()).chain(command_tail.iter()),
                            print_command,
                        );
                    }
                }
            },
            #[cfg(feature = "qecp_integrate")]
            Commands::Qecp(benchmark_parameters) => {
                benchmark_parameters.run().unwrap();
            }
        }
    }
}

pub fn execute_in_cli<'a>(iter: impl Iterator<Item = &'a String> + Clone, print_command: bool) {
    if print_command {
        print!("[command]");
        for word in iter.clone() {
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
        primal_dual_config: serde_json::Value,
    ) -> Box<dyn PrimalDualSolver> {
        match self {
            Self::DualRTL => {
                assert_eq!(primal_dual_config, json!({}));
                Box::new(SolverDualRTL::new(initializer))
            }
            Self::PrimalEmbedded => {
                assert_eq!(primal_dual_config, json!({}));
                Box::new(SolverPrimalEmbedded::new(initializer))
            }
            _ => unimplemented!(),
        }
    }
}
