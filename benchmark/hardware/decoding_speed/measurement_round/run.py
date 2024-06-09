import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *
from hardware.frequency_optimization.circuit_level_various_T.run import configurations
from hardware.decoding_speed.circuit_level_common import *

this_dir = os.path.dirname(os.path.abspath(__file__))

# SAMPLES = 10_000  # draft
SAMPLES = 1_000_000  # final

p = 0.001

if __name__ == "__main__":
    data = []
    for use_layer_fusion in [True, False]:
        for configuration in configurations:
            d = configuration.d
            noisy_measurements = configuration.noisy_measurements
            suffix = "fusion" if use_layer_fusion else "batch"
            benchmarker = DecodingSpeedBenchmarker(
                this_dir=this_dir,
                configuration=configuration,
                p=p,
                samples=SAMPLES,
                use_layer_fusion=use_layer_fusion,
                name_suffix=f"_{suffix}",
            )
            result = benchmarker.run()
            data.append(result)
