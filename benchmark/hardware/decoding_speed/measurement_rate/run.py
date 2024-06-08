import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *
from hardware.frequency_optimization.circuit_level_final.run import (
    Configuration as CircuitLevelFinalConfig,
)
from hardware.decoding_speed.circuit_level_common import *

this_dir = os.path.dirname(os.path.abspath(__file__))

SAMPLES = 10_000  # draft
# SAMPLES = 1_000_000  # final

p = 0.001
# measurement cycle in nanoseconds, from 100ns to 10us
points_per_10 = 5
measurement_cycle_ns_vec = [
    int(100 * (10 ** (i / points_per_10))) for i in range(points_per_10 * 2 + 1)
]

if __name__ == "__main__":
    data = []
    for d in d_vec:
        for measurement_cycle_ns in measurement_cycle_ns_vec:
            benchmarker = DecodingSpeedBenchmarker(
                this_dir=this_dir,
                configuration=CircuitLevelFinalConfig(d=d),
                p=p,
                samples=SAMPLES,
                use_layer_fusion=True,
                name_suffix=f"_mc_{measurement_cycle_ns}",
                measurement_cycle_ns=measurement_cycle_ns,
            )
            result = benchmarker.run()
            data.append(result)
    # save_data(data, this_dir)
    # plot_data(this_dir)
