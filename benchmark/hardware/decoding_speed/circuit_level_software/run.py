import os, sys, git

# better performance, still safe
os.environ["FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER"] = "TRUE"

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *
from hardware.frequency_optimization.circuit_level_final.run import (
    Configuration as CircuitLevelFinalConfig,
)
from hardware.decoding_speed.circuit_level_common import *

from micro_util import *

print(fusion_benchmark_dir)
sys.path.insert(0, fusion_benchmark_dir)
import util as fusion_util
from util import fusion_blossom_benchmark_command

compile_code_if_necessary()

this_dir = os.path.dirname(os.path.abspath(__file__))
profile_dir = os.path.join(this_dir, "tmp-profile")
os.makedirs(profile_dir, exist_ok=True)

# SAMPLES = 10_000  # draft
SAMPLES = 1_00_000  # final

RUN_REPEAT = 10  # use the same set of syndrome to repeat; better averaging value

if __name__ == "__main__":
    data = []
    for d in d_vec:
        noisy_measurements = d - 1
        latency_vec = []
        for p in p_vec:
            # first generate syndrome data
            benchmarker = DecodingSpeedBenchmarker(
                this_dir=this_dir,
                configuration=CircuitLevelFinalConfig(d=d),
                p=p,
                samples=SAMPLES,
                use_layer_fusion=False,
            )
            graph_builder = benchmarker.get_graph_builder()
            defect_file_path = graph_builder.defect_file_path()
            if os.path.exists(defect_file_path):
                graph_builder.assert_defects_file_samples(SAMPLES)
            else:
                graph_builder.build()
            syndrome_file_path = graph_builder.syndrome_file_path()
            assert os.path.exists(syndrome_file_path)

            # run the simulation
            distribution = TimeDistribution()
            for run_index in range(RUN_REPEAT):
                benchmark_profile_path = os.path.join(
                    profile_dir, f"{graph_builder.name}_r{run_index}.profile"
                )
                if os.path.exists(benchmark_profile_path):
                    print(
                        f"[warning] found existing profile (if you think it's stale, delete it and rerun): {benchmark_profile_path}"
                    )
                else:
                    command = fusion_blossom_benchmark_command(
                        d=d,
                        p=p,
                        total_rounds=SAMPLES,
                        noisy_measurements=noisy_measurements,
                    )
                    command += ["--code-type", "error-pattern-reader"]
                    command += [
                        "--code-config",
                        f'{{"filename":"{syndrome_file_path}"}}',
                    ]
                    command += ["--primal-dual-type", "serial"]
                    command += ["--verifier", "none"]
                    command += ["--benchmark-profiler-output", benchmark_profile_path]
                    print(command)
                    stdout, returncode = run_command_get_stdout(command)
                    print("\n" + stdout)
                    assert returncode == 0, "command fails..."

                profile = Profile(benchmark_profile_path)
                distribution.add_profile(profile)
            latency_vec.append(distribution)
        data.append(latency_vec)
        save_data(data, this_dir)
    plot_data(this_dir)
