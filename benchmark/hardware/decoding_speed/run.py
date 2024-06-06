import os, sys, git, shutil
from typing import Protocol
from dataclasses import dataclass, field

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from micro_util import *
from vivado_builder import *
from defects_generator import *
from frequency_explorer import *
from main_benchmark_decoding import *
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
    p: float
    samples: int = 100
    # using either stream (layer fusion) or batch decoding
    use_layer_fusion: bool = False
    measurement_cycle_ns: int = 1000
    multiple_fusion: bool = True
    enable_detailed_print: bool = False

    def get_graph_builder(self) -> MicroBlossomGraphBuilder:
        graph_builder = self.configuration.get_graph_builder()
        graph_builder.test_syndrome_count = self.samples
        graph_builder.graph_folder = os.path.join(this_dir, "tmp-syndrome")
        graph_builder.name += f"_p_{self.p}_N_{self.samples}"
        graph_builder.p = self.p
        return graph_builder

    def tty_result_path(self) -> str:
        graph_builder = self.get_graph_builder()
        tty_result_path = os.path.join(this_dir, "tmp-tty")
        if not os.path.exists(tty_result_path):
            os.mkdir(tty_result_path)
        return os.path.join(tty_result_path, f"{graph_builder.name}.txt")

    def generate_defects(self) -> str:
        graph_builder = self.get_graph_builder()
        print(graph_builder)
        # first check whether the file already exists
        defects_generator = LargeDefectsGenerator(graph_builder)
        return defects_generator.generate()

    def run(self) -> BenchmarkDecodingResult:
        # copy the defects to the folder
        defects_file_path = self.generate_defects()
        dest_file_path = os.path.join(embedded_dir, "embedded.defects")
        shutil.copyfile(defects_file_path, dest_file_path)
        # build the project
        project = self.configuration.optimized_project()
        project.create_vivado_project(update=True)  # update c files
        make_env = os.environ.copy()
        assert "USE_LAYER_FUSION" not in make_env
        if self.use_layer_fusion:
            make_env["USE_LAYER_FUSION"] = "1"
        make_env["MEASUREMENT_CYCLE_NS"] = f"{self.measurement_cycle_ns}"
        graph = SingleGraph.from_file(project.graph_builder.graph_file_path())
        make_env["NUM_LAYER_FUSION"] = f"{graph.layer_fusion.num_layers}"
        assert "DISABLE_MULTIPLE_FUSION" not in make_env
        if not self.multiple_fusion:
            make_env["DISABLE_MULTIPLE_FUSION"] = "1"
        if not self.enable_detailed_print:
            make_env["DISABLE_DETAIL_PRINT"] = "1"
        make_env["EMBEDDED_BLOSSOM_MAIN"] = "benchmark_decoding"
        project.build_embedded_binary(make_env)
        project.build_vivado_project(force_recompile_binary=True)
        assert not project.timing_sanity_check_failed()
        print("running application")
        tty_output = project.run_application()
        with open(self.tty_result_path(), "w", encoding="utf8") as f:
            f.write(tty_output)
        return BenchmarkDecodingResult.from_tty_output(tty_output)


if __name__ == "__main__":
    # debug test
    for d in range(3, 14, 2):
        benchmarker = DecodingSpeedBenchmarker(
            CircuitLevelFinalConfig(d=d), p=0.001, samples=10000
        )
        benchmarker.run()
    # with open("tmp-tty/d_3_p_0.001_N_100.txt") as f:
    #     result = BenchmarkDecodingResult.from_tty_output(f.read())
    #     print(result)
    #     print(result.latency.to_line())
