import os, sys, git, json

# better performance, still safe
os.environ["FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER"] = "TRUE"

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *
from defects_generator import *
from hardware.frequency_optimization.circuit_level_final.run import (
    Configuration as CircuitLevelFinalConfig,
)
from hardware.frequency_optimization.circuit_level_no_offloading.run import (
    Configuration as CircuitLevelNoOffloadingConfig,
)
from micro_util import *

print(fusion_benchmark_dir)
sys.path.insert(0, fusion_benchmark_dir)
import util as fusion_util
from util import fusion_blossom_benchmark_command

this_dir = os.path.dirname(os.path.abspath(__file__))
profile_dir = os.path.join(this_dir, "tmp-profile")
os.makedirs(profile_dir, exist_ok=True)

MIN_SAMPLES = 10_000
MAX_SAMPLES = 10_000_000_000
ACCUMULATE_LOGICAL_ERRORS = 1000  # accumulate how many logical errors before return
# ACCUMULATE_LOGICAL_ERRORS = 0.01  # for debugging

d_vec = [9]
p_vec = [0.001]

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
            benchmarker = DecodingSpeedBenchmarker(
                this_dir=this_dir,
                configuration=CircuitLevelFinalConfig(d=d),
                p=p,
                samples=samples,
                use_layer_fusion=False,
            )
            graph_builder = benchmarker.get_graph_builder()
            # first check whether the file already exists
            defects_generator = LargeDefectsGenerator(
                graph_builder, generate_syndrome_max_N=10_000_000
            )
            defects_generator.generate(keep_syndrome_files=True)
            chunks = defects_generator.chunks()
            chunk_length = defects_generator.chunk_length()
            if chunks is None:
                syndrome_files = [graph_builder.syndrome_file_path()]
            else:
                base = graph_builder.syndrome_file_path()
                assert base[-10:] == ".syndromes"
                syndrome_files = [
                    base[:-10] + f"_chunk_{chunk}.syndromes" for chunk in range(chunks)
                ]
            for syndrome_file_path in syndrome_files:
                assert os.path.exists(syndrome_file_path)
            # run the result for each chunk
            benchmark_profile_paths = []
            for chunk, syndrome_file_path in enumerate(syndrome_files):
                benchmark_profile_path = os.path.join(
                    profile_dir, f"{graph_builder.name}_{chunk}.profile"
                )
                benchmark_profile_paths.append(benchmark_profile_path)
                if os.path.exists(benchmark_profile_path):
                    print(
                        "[warning] found existing profile (if you think it's stale, delete it and rerun)"
                    )
                else:
                    command = fusion_blossom_benchmark_command(
                        d=d,
                        p=p,
                        total_rounds=chunk_length,
                        noisy_measurements=d - 1,
                    )
                    command += ["--code-type", "error-pattern-reader"]
                    command += [
                        "--code-config",
                        f'{{"filename":"{syndrome_file_path}"}}',
                    ]
                    command += ["--primal-dual-type", "serial"]
                    command += ["--verifier", "none"]
                    command += [
                        "--benchmark-profiler-output",
                        benchmark_profile_path,
                    ]
                    print(command)
                    stdout, returncode = run_command_get_stdout(command)
                    print("\n" + stdout)
                    assert returncode == 0, "command fails..."
            # gather the results
            distribution = TimeDistribution()
            for benchmark_profile_path in benchmark_profile_paths:
                print(f"reading profile data: {benchmark_profile_path}")
                profile = Profile(
                    benchmark_profile_path,
                    apply_entries=lambda x: {
                        "events": {"decoded": x["events"]["decoded"]}
                    },
                )
                distribution.add_profile(profile)
                del profile  # save memory
            filename = f"d_{d}_p_{p}_software.txt"
            with open(os.path.join(this_dir, filename), "w", encoding="utf8") as f:
                f.write(distribution.to_line())
