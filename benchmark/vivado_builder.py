import os
import sys
import subprocess
import shutil
from datetime import datetime
from dataclasses import dataclass, field

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
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
if True:
    from micro_util import *
    from build_micro_blossom import main as build_micro_blossom_main

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
    from get_ttyoutput import get_ttyoutput


@dataclass
class MicroBlossomGraphBuilder:
    """build the graph using QEC-Playground"""

    graph_folder: str
    name: str
    d: int
    p: float
    noisy_measurements: int
    max_half_weight: int
    code_type: str = "rotated-planar-code"
    noise_model: str = "stim-noise-model"
    only_stab_z: bool = True
    use_combined_probability: bool = True
    test_syndrome_count: int = 100

    def decoder_config(self):
        return {
            "only_stab_z": self.only_stab_z,
            "use_combined_probability": self.use_combined_probability,
            "skip_decoding": True,
            "max_half_weight": self.max_half_weight,
        }

    def graph_file_path(self) -> str:
        return os.path.join(self.graph_folder, f"{self.name}.json")

    def syndrome_file_path(self) -> str:
        return os.path.join(self.graph_folder, f"{self.name}.syndromes")

    def run(self) -> None:
        assert os.path.exists(self.graph_folder)

        # first create the syndrome file
        syndrome_file_path = self.syndrome_file_path()
        if not os.path.exists(syndrome_file_path):
            command = fusion_blossom_qecp_generate_command(
                d=self.d,
                p=self.p,
                total_rounds=self.test_syndrome_count,
                noisy_measurements=self.noisy_measurements,
            )
            command += ["--code-type", self.code_type]
            command += ["--noise-model", self.noise_model]
            command += [
                "--decoder",
                "fusion",
                "--decoder-config",
                json.dumps(self.decoder_config(), separators=(",", ":")),
            ]
            command += [
                "--debug-print",
                "fusion-blossom-syndrome-file",
                "--fusion-blossom-syndrome-export-filename",
                syndrome_file_path,
            ]
            command += ["--parallel", f"0"]  # use all cores
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

        # then generate the graph json
        graph_file_path = self.graph_file_path()
        if not os.path.exists(graph_file_path):
            command = micro_blossom_command() + ["parser"]
            command += [syndrome_file_path]
            command += ["--graph-file", graph_file_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."


@dataclass
class MicroBlossomAxi4Builder:
    graph_builder: MicroBlossomGraphBuilder

    project_folder: str
    name: str
    clock_frequency: float = 200  # in MHz
    clock_divide_by: int = 2
    inject_registers: str = ""  # e.g. "offload", "offload,update3"

    def hardware_proj_dir(self) -> str:
        return os.path.join(self.project_folder, self.name)

    def prepare_graph(self):
        self.graph_builder.run()

    def create_vivado_project(self):
        if not os.path.exists(self.hardware_proj_dir()):
            parameters = ["--name", self.name]
            parameters += ["--path", self.project_folder]
            parameters += ["--clock-frequency", f"{self.clock_frequency}"]
            parameters += ["--clock-divide-by", f"{self.clock_divide_by}"]
            parameters += ["--graph", self.graph_builder.graph_file_path()]
            parameters += ["--inject-registers"] + self.inject_registers
            build_micro_blossom_main(parameters)

    def build_vivado_project(self): ...

    def run(self):
        self.prepare_graph()
        self.create_vivado_project()
        self.build_vivado_project()
