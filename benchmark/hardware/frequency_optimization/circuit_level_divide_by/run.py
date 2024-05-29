import os, sys, git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *
from frequency_explorer import *


@dataclass
class Configuration:
    d: int
    f: float = 250
    clock_divide_by: int = 2  # start exploring with this value

    def name(self) -> str:
        return f"d_{self.d}"


this_dir = os.path.dirname(os.path.abspath(__file__))
frequency_log_dir = os.path.join(this_dir, "frequency_log")
if not os.path.exists(frequency_log_dir):
    os.mkdir(frequency_log_dir)


configurations = [
    Configuration(d=3),
    Configuration(d=5),
    Configuration(d=7),
    Configuration(d=9),
    Configuration(d=11),
    Configuration(d=13),
    Configuration(d=15),
    Configuration(d=17),
]


def get_project(
    configuration: Configuration, divide_by: int
) -> MicroBlossomAxi4Builder:
    graph_builder = MicroBlossomGraphBuilder(
        graph_folder=os.path.join(this_dir, "tmp-graph"),
        name=configuration.name(),
        d=configuration.d,
        p=0.001,
        noisy_measurements=configuration.d - 1,
        max_half_weight=7,  # higher weights represents the case for large code distances
        visualize_graph=True,
    )
    return MicroBlossomAxi4Builder(
        graph_builder=graph_builder,
        name=configuration.name() + f"_c{divide_by}",
        clock_frequency=configuration.f,
        clock_divide_by=divide_by,
        project_folder=os.path.join(this_dir, "tmp-project"),
    )


for configuration in configurations:

    def compute_next_minimum_divide_by(frequency: int) -> int:
        project = get_project(configuration, frequency)
        project.build()
        return project.next_minimum_clock_divide_by()

    explorer = ClockDivideByExplorer(
        compute_next_minimum_divide_by=compute_next_minimum_divide_by,
        log_filepath=os.path.join(frequency_log_dir, configuration.name() + ".txt"),
    )

    best_divide_by = explorer.optimize()
    print(f"{configuration.name()}: {best_divide_by}")

    project = get_project(configuration, best_divide_by)
