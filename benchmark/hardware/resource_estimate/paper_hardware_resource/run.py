import os
import sys
import subprocess
import sys

git_root_dir = (
    subprocess.run(
        "git rev-parse --show-toplevel",
        cwd=os.path.dirname(os.path.abspath(__file__)),
        shell=True,
        check=True,
        capture_output=True,
    )
    .stdout.decode(sys.stdout.encoding)
    .strip(" \r\n")
)
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
if True:
    from micro_util import *
    from vivado_project import VivadoProject

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
this_dir = os.path.dirname(os.path.abspath(__file__))
hardware_dir = os.path.join(this_dir, "hardware")


@dataclass
class Configuration:
    name: str
    d_vec: list[int]
    f: float = 10.0  # 10 MHz
    p: float = 0.001
    noise_model: str = "stim-noise-model"
    max_half_weight: int = 7
    fix_noisy_measurements: Optional[int] = None
    scala_parameters: Optional[list[str]] = None

    def noisy_measurements(self, d: int) -> int:
        if self.fix_noisy_measurements is not None:
            return self.fix_noisy_measurements
        return d - 1

    def hardware_proj_name(self, d) -> str:
        return f"{self.name}_d_{d}"

    def hardware_proj_dir(self, d) -> str:
        return os.path.join(hardware_dir, self.hardware_proj_name(d))


configurations = [
    Configuration(
        name="code_capacity",
        d_vec=[3, 5, 7, 9, 11, 15, 21, 27] + [35, 51, 69, 81],
        noise_model="phenomenological",
        fix_noisy_measurements=0,
        max_half_weight=1,
    ),
    Configuration(
        name="phenomenological",
        d_vec=[3, 5, 7, 9, 11, 13, 15, 17],
        noise_model="phenomenological",
        max_half_weight=1,
    ),
    Configuration(
        name="circuit",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
    ),
    Configuration(
        name="circuit_offload",
        d_vec=[3, 5, 7, 9, 11, 13, 15],
        scala_parameters=["--support-offloading"],
    ),
    # Configuration(name="circuit_fusion",d_vec=[3, 5, 7, 9, 11, 13, 15]),
]


def main(config: Configuration):
    name = config.name
    frequency = config.f
    p = config.p

    # first check that all the implementations are ready
    for d in config.d_vec:
        assert os.path.exists(config.hardware_proj_dir(d)), "run perpare.py first"
        assert os.path.exists(
            os.path.join(
                config.hardware_proj_dir(d), f"{config.hardware_proj_name(d)}.xsa"
            )
        ), "the implementation failed, please rerun it or remove it from the configuration"

    # then run the resource report if the file does not exist
    report_filename = os.path.join(this_dir, f"resource_{name}.txt")
    # run resource report on each of the project
    results = [
        "# <d> <clb LUTs> <clb_percent> <registers> <reg_percent>"
        + " <#V> <#E> <#pre-matcher>"
    ]
    for d in config.d_vec:
        vivado = VivadoProject(config.hardware_proj_dir(d))
        report = vivado.report_impl_utilization(force_regenerate=False)
        clb_luts = report.netlist_logic.clb_luts
        registers = report.netlist_logic.registers
        # also read the number of vertices in the graph and the number of edges
        graph_file_path = os.path.join(hardware_dir, f"{name}_d_{d}.json")
        graph = SingleGraph.from_file(graph_file_path)
        pre_match_num = graph.effective_offloader_num(
            support_offloading=(
                config.scala_parameters is not None
                and "--support-offloading" in config.scala_parameters
            ),
            support_layer_fusion=(
                config.scala_parameters is not None
                and "--support-layer-fusion" in config.scala_parameters
            ),
        )
        results.append(
            f"{d} {clb_luts.used} {clb_luts.util_percent} {registers.used} {registers.util_percent}"
            + f" {graph.vertex_num} {len(graph.weighted_edges)} {pre_match_num}"
        )
    with open(report_filename, "w", encoding="utf8") as f:
        f.write("\n".join(results))


if __name__ == "__main__":
    errors = []
    for configuration in configurations:
        main(configuration)
