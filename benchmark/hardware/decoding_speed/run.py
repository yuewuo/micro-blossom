import os, sys, git, math
from typing import Protocol
from dataclasses import dataclass, field

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *
from defects_generator import *
from frequency_explorer import *
from hardware.frequency_optimization.circuit_level_final.run import (
    Configuration as CircuitLevelFinalConfig,
)

this_dir = os.path.dirname(os.path.abspath(__file__))


class Configuration(Protocol):
    def get_graph_builder(self) -> MicroBlossomGraphBuilder: ...
    def optimized_project(self) -> MicroBlossomAxi4Builder: ...


@dataclass
class DecodingSpeedBenchmarker:
    configuration: Configuration

    def generate_defects(self, p: float | None = None, N: int = 10000) -> str:
        graph_builder = self.configuration.get_graph_builder()
        graph_builder.test_syndrome_count = N
        graph_builder.graph_folder = os.path.join(this_dir, "tmp-syndrome")
        graph_builder.name += f"_p_{p}_N_{N}"
        if p is not None:
            graph_builder.p = p
        print(graph_builder)
        # first check whether the file already exists
        defects_generator = LargeDefectsGenerator(graph_builder)
        return defects_generator.generate()

    def run(self, N: int = 100000):
        project = self.configuration.optimized_project()
        graph_builder = project.graph_builder

        print(graph_builder.d)


if __name__ == "__main__":
    # debug test
    benchmarker = DecodingSpeedBenchmarker(CircuitLevelFinalConfig(d=9))
    defect_file_path = benchmarker.generate_defects(p=0.001, N=200000)
    print(defect_file_path)
    benchmarker.run()
