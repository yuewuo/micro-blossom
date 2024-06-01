import os, sys, git, math
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *
from frequency_explorer import *


@dataclass
class Configuration:
    inject_registers: list[str]
    broadcast_delay: int = 0

    frequency: float = 150

    def name(self) -> str:
        registers = "_".join(self.inject_registers)
        if registers == "":
            registers = "none"
        return f"r_{registers}_b{self.broadcast_delay}"


this_dir = os.path.dirname(os.path.abspath(__file__))
frequency_log_dir = os.path.join(this_dir, "frequency_log")
if not os.path.exists(frequency_log_dir):
    os.mkdir(frequency_log_dir)


graph_builder = MicroBlossomGraphBuilder(
    graph_folder=os.path.join(this_dir, "tmp-graph"),
    name="d_9_circuit_level_full",
    d=9,
    p=0.001,
    noisy_measurements=9 - 1,
    max_half_weight=7,
    visualize_graph=True,
)

configurations = [
    Configuration(inject_registers=[]),
    Configuration(inject_registers=[], broadcast_delay=1, frequency=90),
    Configuration(inject_registers=["execute"], frequency=90),
    Configuration(inject_registers=["execute"], broadcast_delay=1, frequency=120),
    Configuration(inject_registers=["execute", "update"], frequency=120),
    Configuration(
        inject_registers=["execute", "update"], broadcast_delay=1, frequency=150
    ),
]


def get_project(
    configuration: Configuration, frequency: int
) -> MicroBlossomAxi4Builder:
    return MicroBlossomAxi4Builder(
        graph_builder=graph_builder,
        name=configuration.name() + f"_sf{frequency}",
        clock_frequency=frequency,
        project_folder=os.path.join(this_dir, "tmp-project"),
        broadcast_delay=configuration.broadcast_delay,
        inject_registers=configuration.inject_registers,
    )


results = ["# <name> <best frequency/MHz>"]
for configuration in configurations:

    def compute_next_maximum_frequency(frequency: int) -> int | None:
        project = get_project(configuration, frequency)
        project.build()
        return project.next_maximum_frequency()

    explorer = FrequencyExplorer(
        compute_next_maximum_frequency=compute_next_maximum_frequency,
        log_filepath=os.path.join(frequency_log_dir, configuration.name() + ".txt"),
        max_frequency=configuration.frequency,
    )

    best_frequency = explorer.optimize()
    print(f"{configuration.name()}: {best_frequency}MHz")
    results.append(f"{configuration.name()} {best_frequency}")

    # project = get_project(configuration, best_frequency)

with open("best_slow_frequencies.txt", "w", encoding="utf8") as f:
    f.write("\n".join(results))
