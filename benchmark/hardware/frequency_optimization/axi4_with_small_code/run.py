import os, sys, git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *
from frequency_explorer import *


@dataclass
class Configuration:
    context_depth: int
    f: float = 300  # start frequency: searching from this frequency
    clock_divide_by: int = 4  # to minimize the timing effect on the axi4 bus

    def name(self) -> str:
        return f"context_{self.context_depth}"


this_dir = os.path.dirname(os.path.abspath(__file__))
frequency_log_dir = os.path.join(this_dir, "frequency_log")
if not os.path.exists(frequency_log_dir):
    os.mkdir(frequency_log_dir)


graph_builder = MicroBlossomGraphBuilder(
    graph_folder=os.path.join(this_dir, "tmp-graph"),
    name="d3_weighted",
    d=3,
    p=0.01,
    noisy_measurements=0,
    max_half_weight=50,  # higher weights represents the case for large code distances
    noise_model="phenomenological",
    visualize_graph=True,
)

configurations = [
    Configuration(context_depth=1),
    Configuration(context_depth=2),
    Configuration(context_depth=4),
    Configuration(context_depth=8),
    Configuration(context_depth=16),
    Configuration(context_depth=32),
    Configuration(context_depth=64),
    Configuration(context_depth=128),
    Configuration(context_depth=256),
    Configuration(context_depth=512),
    Configuration(context_depth=1024),
]


def get_project(
    configuration: Configuration, frequency: int
) -> MicroBlossomAxi4Builder:
    return MicroBlossomAxi4Builder(
        graph_builder=graph_builder,
        name=configuration.name() + f"_f{frequency}",
        clock_frequency=frequency,
        clock_divide_by=configuration.clock_divide_by,
        project_folder=os.path.join(this_dir, "tmp-project"),
        context_depth=configuration.context_depth,
    )


results = ["# <context depth> <best frequency/MHz>"]
for configuration in configurations:

    def compute_next_maximum_frequency(frequency: int) -> int:
        project = get_project(configuration, frequency)
        project.build()
        return project.next_maximum_frequency()

    explorer = FrequencyExplorer(
        compute_next_maximum_frequency=compute_next_maximum_frequency,
        log_filepath=os.path.join(frequency_log_dir, configuration.name() + ".txt"),
    )

    best_frequency = explorer.optimize()
    print(f"{configuration.name()}: {best_frequency}MHz")
    results.append(f"{configuration.context_depth} {best_frequency}")

    # project = get_project(configuration, best_frequency)

with open("best_frequencies.txt", "w", encoding="utf8") as f:
    f.write("\n".join(results))
