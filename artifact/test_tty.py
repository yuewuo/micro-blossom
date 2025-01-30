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


if __name__ == "__main__":
    d = 3
    p = 1e-4
    benchmarker = DecodingSpeedBenchmarker(
        this_dir=this_dir,
        configuration=CircuitLevelFinalConfig(d=d),
        p=p,
        samples=10_000,
        use_layer_fusion=True,
    )
    result = benchmarker.run()
    print(f"latency: {result.latency}")
