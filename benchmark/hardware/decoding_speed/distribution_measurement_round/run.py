import os, sys, git, json

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *
from hardware.frequency_optimization.circuit_level_various_T.run import configurations

this_dir = os.path.dirname(os.path.abspath(__file__))

MIN_SAMPLES = 10_000
MAX_SAMPLES = 10_000_000_000
ACCUMULATE_LOGICAL_ERRORS = 100  # accumulate how many logical errors before return
# ACCUMULATE_LOGICAL_ERRORS = 0.01  # for debugging

d = 9
p = 0.001
pL = 4.075622523830063e-06  # see ../distribution


if __name__ == "__main__":
    samples = int(ACCUMULATE_LOGICAL_ERRORS / pL)
    samples = min(samples, MAX_SAMPLES)
    samples = max(samples, MIN_SAMPLES)
    print(f"logical error rate: {pL}")
    print(f"running {samples} samples")
    for use_layer_fusion in [True, False]:
        suffix = "fusion" if use_layer_fusion else "batch"
        results = ["# <d> <measurement rounds> <p> <average decoding time>"]

        for configuration in configurations:
            d = configuration.d
            nm = configuration.noisy_measurements
            benchmarker = DecodingSpeedBenchmarker(
                this_dir=this_dir,
                configuration=configuration,
                p=p,
                samples=samples,
                use_layer_fusion=use_layer_fusion,
                name_suffix=f"_{suffix}",
            )
            result = benchmarker.run()
            results.append(f"{d} {nm+1} {p} {result.latency.average_latency()}")
            # also record the distribution
            filename = f"{suffix}_{nm+1}.txt"
            with open(filename, "w", encoding="utf8") as f:
                f.write(result.latency.to_line())

        with open(f"{suffix}.txt", "w", encoding="utf8") as f:
            f.write("\n".join(results))
