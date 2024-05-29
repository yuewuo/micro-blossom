import os, sys, git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *


@dataclass
class Configuration:
    context_depth: int
    f: float = 300  # start frequency: searching from this frequency

    def name(self) -> str:
        return f"context_{self.context_depth}"


this_dir = os.path.dirname(os.path.abspath(__file__))


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

for configuration in configurations:
    project = MicroBlossomAxi4Builder(
        graph_builder=graph_builder,
        name=configuration.name(),
        clock_frequency=configuration.f,
        project_folder=os.path.join(this_dir, "tmp-project"),
    )

    project.build()
    project.create_timing_report()
