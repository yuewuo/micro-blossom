import os, sys, git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from micro_util import *
from vivado_builder import *
from frequency_explorer import *

this_dir = os.path.dirname(os.path.abspath(__file__))


@dataclass
class Configuration(OptimizableConfiguration):
    d: int
    frequency: int | None = None

    def init_frequency(self) -> int:
        return HeuristicFrequencyCircuitLevel.of(self.d)

    def frequency_log_dir(self) -> str:
        return os.path.join(this_dir, "frequency_log")

    def name(self) -> str:
        return f"d_{self.d}"

    def get_graph_builder(self) -> MicroBlossomGraphBuilder:
        return MicroBlossomGraphBuilder(
            graph_folder=os.path.join(this_dir, "tmp-graph"),
            name=self.name(),
            d=self.d,
            p=0.001,
            noisy_measurements=self.d - 1,
            max_half_weight=7,
        )

    def get_project(self, frequency: int | None = None) -> MicroBlossomAxi4Builder:
        if frequency is None:
            frequency = self.frequency
        return MicroBlossomAxi4Builder(
            graph_builder=self.get_graph_builder(),
            name=self.name() + f"_f{frequency}",
            clock_frequency=frequency,
            project_folder=os.path.join(this_dir, "tmp-project"),
            inject_registers=["execute", "update"],
            support_offloading=False,
            support_layer_fusion=True,
            support_load_stall_emulator=True,
        )


configurations = [Configuration(d=d) for d in range(3, 16, 2)]


def main():
    results = ["# <name> <best frequency/MHz> <estimated frequency/MHz> <vertex num>"]

    for configuration in configurations:

        optimized = configuration.optimized_project()
        print(
            f"{configuration.name()}: {optimized.clock_frequency}MHz "
            + f"(estimated max = {optimized.estimate_maximum_frequency()}MHz)"
        )
        graph_file_path = optimized.graph_builder.graph_file_path()
        graph = SingleGraph.from_file(graph_file_path)
        results.append(
            f"{configuration.name()} {optimized.clock_frequency} {optimized.estimate_maximum_frequency()} {graph.vertex_num}"
        )

    with open("best_frequencies.txt", "w", encoding="utf8") as f:
        f.write("\n".join(results))


if __name__ == "__main__":
    main()
