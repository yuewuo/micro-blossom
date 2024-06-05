import os, sys, git, math
from dataclasses import dataclass, field

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *
from frequency_explorer import *


@dataclass
class Configuration:
    d: int

    def init_frequency(self) -> int:
        return HeuristicFrequencyCircuitLevel.of(self.d)

    def name(self) -> str:
        return f"d_{self.d}"

    def get_project(self, frequency: int | None) -> MicroBlossomAxi4Builder:
        if frequency is None:
            frequency = self.frequency
        graph_builder = MicroBlossomGraphBuilder(
            graph_folder=os.path.join(this_dir, "tmp-graph"),
            name=f"d_{self.d}",
            d=self.d,
            p=0.001,
            noisy_measurements=self.d - 1,
            max_half_weight=7,
        )
        return MicroBlossomAxi4Builder(
            graph_builder=graph_builder,
            name=self.name() + f"_f{frequency}",
            clock_frequency=frequency,
            project_folder=os.path.join(this_dir, "tmp-project"),
            inject_registers=["execute", "update"],
            support_offloading=False,
            support_layer_fusion=True,
            support_load_stall_emulator=True,
        )


configurations = [Configuration(d=d) for d in range(3, 18, 2)]

this_dir = os.path.dirname(os.path.abspath(__file__))
frequency_log_dir = os.path.join(this_dir, "frequency_log")
if not os.path.exists(frequency_log_dir):
    os.mkdir(frequency_log_dir)


def main() -> list[Configuration]:
    results = ["# <name> <best frequency/MHz>"]
    optimized_configurations = []

    for configuration in configurations:

        def compute_next_maximum_frequency(frequency: int) -> int | None:
            project = configuration.get_project(frequency)
            project.build()
            return project.next_maximum_frequency()

        explorer = FrequencyExplorer(
            compute_next_maximum_frequency=compute_next_maximum_frequency,
            log_filepath=os.path.join(frequency_log_dir, configuration.name() + ".txt"),
            max_frequency=configuration.init_frequency(),
        )

        best_frequency = explorer.optimize()
        print(f"{configuration.name()}: {best_frequency}MHz")
        results.append(f"{configuration.name()} {best_frequency}")

        optimized = Configuration(**configuration.__dict__)
        optimized.frequency = best_frequency
        optimized_configurations.append(optimized)

    with open("best_frequencies.txt", "w", encoding="utf8") as f:
        f.write("\n".join(results))

    return optimized_configurations


if __name__ == "__main__":
    main()
