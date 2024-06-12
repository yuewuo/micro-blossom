import os, sys, git, json

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *
from hardware.frequency_optimization.circuit_level_final.run import (
    Configuration as CircuitLevelFinalConfig,
)
from hardware.frequency_optimization.circuit_level_no_offloading.run import (
    Configuration as CircuitLevelNoOffloadingConfig,
)

this_dir = os.path.dirname(os.path.abspath(__file__))

MIN_SAMPLES = 10_000
MAX_SAMPLES = 10_000_000_000
ACCUMULATE_LOGICAL_ERRORS = 1000  # accumulate how many logical errors before return
# ACCUMULATE_LOGICAL_ERRORS = 0.01  # for debugging

d_vec = [9]
p_vec = [0.001]

constructs = [
    # name, configuration class, use_layer_fusion,
    ("fusion", CircuitLevelFinalConfig, True),
    ("batch", CircuitLevelFinalConfig, False),
    # ("no_offloading", CircuitLevelNoOffloadingConfig, False),
]

logical_error_rate_file = os.path.join(this_dir, "logical_error_rate.json.save")


def estimate_logical_error_rate(d: int, p: float) -> float:
    logical_error_rates = {}
    if os.path.exists(logical_error_rate_file):
        with open(logical_error_rate_file, "r", encoding="utf8") as f:
            logical_error_rates = json.load(f)
    key = f"{d}_{p}"
    if key in logical_error_rates:
        return logical_error_rates[key]
    # calculate the samples to run
    graph_builder = CircuitLevelFinalConfig(d=d).get_graph_builder()
    graph_builder.p = p
    graph_builder.test_syndrome_count = MAX_SAMPLES
    # obtain a fairly accurate logical error rate
    command = graph_builder.get_simulation_command(min_failed_cases=100)
    decoder_config = graph_builder.decoder_config()
    decoder_config["only_stab_z"] = False
    decoder_config["skip_decoding"] = False  # we actually need decoding
    command += [
        "--decoder",
        "fusion",
        "--decoder-config",
        json.dumps(decoder_config, separators=(",", ":")),
    ]
    command += ["--parallel", f"0"]  # use all cores
    print(command)
    stdout, returncode = run_command_get_stdout(command)
    print("\n" + stdout)
    assert returncode == 0, "command fails..."

    full_result = stdout.strip(" \r\n").split("\n")[-1]
    lst = full_result.split(" ")
    total_rounds = int(lst[3])
    error_count = int(lst[4])
    error_rate = float(lst[5])
    confidence_interval = float(lst[7])

    logical_error_rates[key] = error_rate
    with open(logical_error_rate_file, "w", encoding="utf8") as f:
        json.dump(logical_error_rates, f)
    return error_rate


if __name__ == "__main__":
    for d in d_vec:
        for p in p_vec:
            # calculate how many samples to run
            pL = estimate_logical_error_rate(d, p)
            assert pL > 0
            samples = int(ACCUMULATE_LOGICAL_ERRORS / pL)
            samples = min(samples, MAX_SAMPLES)
            samples = max(samples, MIN_SAMPLES)
            print(f"logical error rate: {pL}")
            print(f"running {samples} samples")
            # run evaluation
            for name, configuration_cls, use_layer_fusion in constructs:
                benchmarker = DecodingSpeedBenchmarker(
                    this_dir=this_dir,
                    configuration=configuration_cls(d=d),
                    name_suffix=f"_{name}",
                    p=p,
                    samples=samples,
                    use_layer_fusion=use_layer_fusion,
                )
                result = benchmarker.run()
                filename = f"d_{d}_p_{p}_{name}.txt"
                with open(filename, "w", encoding="utf8") as f:
                    f.write(result.latency.to_line())
