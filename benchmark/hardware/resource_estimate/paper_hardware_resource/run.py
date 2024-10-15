import os, sys, git, argparse
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from micro_util import *
from vivado_builder import *

this_dir = os.path.dirname(os.path.abspath(__file__))


@dataclass
class Configuration:
    base_name: str
    d_vec: list[int]
    d: int | None = None
    f: float = 10.0  # 10 MHz
    p: float = 0.001
    noise_model: str = "stim-noise-model"
    max_half_weight: int = 7
    fix_noisy_measurements: Optional[int] = None
    support_offloading: bool = False
    support_layer_fusion: bool = False
    context_depth: int = 1

    def config_of(self, d: int) -> "Configuration":
        configuration = Configuration(**self.__dict__)
        configuration.d = d
        return configuration

    def noisy_measurements(self) -> int:
        if self.fix_noisy_measurements is not None:
            return self.fix_noisy_measurements
        return self.d - 1

    def name(self) -> str:
        return f"{self.base_name}_d_{self.d}"

    def get_project(self) -> MicroBlossomAxi4Builder:
        graph_builder = MicroBlossomGraphBuilder(
            graph_folder=os.path.join(this_dir, "tmp-graph"),
            name=self.name(),
            d=self.d,
            p=self.p,
            noise_model=self.noise_model,
            noisy_measurements=self.noisy_measurements(),
            max_half_weight=self.max_half_weight,
        )
        return MicroBlossomAxi4Builder(
            graph_builder=graph_builder,
            name=self.name(),
            clock_frequency=self.f,
            project_folder=os.path.join(this_dir, "tmp-project"),
            inject_registers=["execute", "update"],
            support_offloading=self.support_offloading,
            support_layer_fusion=self.support_layer_fusion,
            context_depth=self.context_depth,
        )


configurations = [
    Configuration(
        base_name="code_capacity",
        d_vec=[3, 5, 7, 9, 11, 15, 21, 27] + [35, 51, 69, 81],
        noise_model="phenomenological",
        fix_noisy_measurements=0,
        max_half_weight=1,
    ),
    Configuration(
        base_name="phenomenological",
        d_vec=[3, 5, 7, 9, 11, 13, 15, 17],
        noise_model="phenomenological",
        max_half_weight=1,
    ),
    Configuration(
        base_name="circuit",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
    ),
    Configuration(
        base_name="circuit_offload",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
    ),
    # python3 prepare.py --base-name circuit_fusion
    Configuration(
        base_name="circuit_fusion",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
        support_layer_fusion=True,
    ),
    # python3 prepare.py --base-name circuit_fusion_context_1024
    Configuration(
        base_name="circuit_fusion_context_1024",
        d_vec=[3, 5, 7, 9, 11, 13],
        support_offloading=True,
        support_layer_fusion=True,
        context_depth=1024,
    ),
    # python3 prepare.py --base-name circuit_fusion_context_2
    Configuration(
        base_name="circuit_fusion_context_1024",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
        support_layer_fusion=True,
        context_depth=2,
    ),
    # python3 prepare.py --base-name circuit_fusion_context_4
    Configuration(
        base_name="circuit_fusion_context_1024",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
        support_layer_fusion=True,
        context_depth=4,
    ),
    # python3 prepare.py --base-name circuit_fusion_context_8
    Configuration(
        base_name="circuit_fusion_context_1024",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
        support_layer_fusion=True,
        context_depth=8,
    ),
    # python3 prepare.py --base-name circuit_fusion_context_16
    Configuration(
        base_name="circuit_fusion_context_1024",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
        support_layer_fusion=True,
        context_depth=16,
    ),
    # python3 prepare.py --base-name circuit_fusion_context_32
    Configuration(
        base_name="circuit_fusion_context_1024",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        support_offloading=True,
        support_layer_fusion=True,
        context_depth=32,
    ),
]


def main(config: Configuration):
    # first check that all the implementations are ready
    for d in config.d_vec:
        configuration = config.config_of(d)
        project = configuration.get_project()
        assert project.has_xsa(), "run prepare.py first"

    # then run the resource report if the file does not exist
    report_filename = os.path.join(this_dir, f"resource_{config.base_name}.txt")
    # run resource report on each of the project
    results = [
        "# <d> <clb LUTs> <clb_percent> <registers> <reg_percent>"
        + " <#V> <#E> <#pre-matcher>"
        + " <bram> <bram_percent>"
    ]
    for d in config.d_vec:
        configuration = config.config_of(d)
        project = configuration.get_project()
        vivado = project.get_vivado()
        report = vivado.report_impl_utilization()
        clb_luts = report.netlist_logic.clb_luts
        registers = report.netlist_logic.registers
        bram_tile = report.bram.bram_tile
        # also read the number of vertices in the graph and the number of edges
        graph_file_path = project.graph_builder.graph_file_path()
        graph = SingleGraph.from_file(graph_file_path)
        pre_match_num = graph.effective_offloader_num(
            support_offloading=config.support_offloading,
            support_layer_fusion=config.support_layer_fusion,
        )
        results.append(
            f"{d} {clb_luts.used} {clb_luts.util_percent} {registers.used} {registers.util_percent}"
            + f" {graph.vertex_num} {len(graph.weighted_edges)} {pre_match_num}"
            + f" {bram_tile.used} {bram_tile.util_percent}"
        )
    with open(report_filename, "w", encoding="utf8") as f:
        f.write("\n".join(results))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Estimate Resource Usage")
    parser.add_argument(
        "--base-name",
        help="if provided, only run the matched configuration",
    )
    args = parser.parse_args()

    errors = []
    for configuration in configurations:
        if args.base_name is None or args.base_name == configuration.base_name:
            main(configuration)
