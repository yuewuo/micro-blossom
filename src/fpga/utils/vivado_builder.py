import os, sys, git, subprocess, math
from datetime import datetime
from dataclasses import dataclass, field

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from micro_util import *
from get_ttyoutput import get_ttyoutput
from build_micro_blossom import *
from vivado_project import VivadoProject


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
    transform_graph: bool = True
    visualize_graph: bool = False

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

    def build(self) -> None:
        graph_file_path = self.graph_file_path()
        if os.path.exists(graph_file_path):
            return

        if not os.path.exists(self.graph_folder):
            os.mkdir(self.graph_folder)

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

            # merge two side of the virtual vertices to reduce resource usage
            if self.transform_graph:
                if self.code_type == "rotated-planar-code":
                    command = micro_blossom_command() + [
                        "transform-syndromes",
                        syndrome_file_path,
                        syndrome_file_path,
                        "qecp-rotated-planar-code",
                        f"{self.d}",
                    ]
                    stdout, returncode = run_command_get_stdout(command)
                    print("\n" + stdout)
                    assert returncode == 0, "command fails..."
                else:
                    raise Exception(f"transform not implemented for ${self.code_type}")

            if self.visualize_graph:
                command = fusion_blossom_command() + [
                    "visualize-syndromes",
                    syndrome_file_path,
                    "--visualizer-filename",
                    f"micro_blossom_{self.name}.json",
                ]
                stdout, returncode = run_command_get_stdout(command)
                print("\n" + stdout)
                assert returncode == 0, "command fails..."

        # then generate the graph json
        if not os.path.exists(graph_file_path):
            command = micro_blossom_command() + ["parser"]
            command += [syndrome_file_path]
            command += ["--graph-file", graph_file_path]
            # at the end of the file, transform the graph that is automatically generated
            if self.transform_graph:
                if self.code_type == "rotated-planar-code":
                    command += ["qecp-rotated-planar-code", f"{self.d}"]
                else:
                    raise Exception(f"transform not implemented for ${self.code_type}")
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
    clock_divide_by: float = 1.0
    overwrite: bool = True
    broadcast_delay: int = 0
    convergecast_delay: int = 1
    context_depth: int = 1
    hard_code_weights: bool = True
    support_add_defect_vertex: bool = True
    support_offloading: bool = False
    support_layer_fusion: bool = False
    # e.g. ["offload"], ["offload", "update3"]
    inject_registers: list[str] | str = field(default_factory=lambda: [])

    # not none after
    project_builder: MicroBlossomProjectBuilder | None = None

    def hardware_proj_dir(self) -> str:
        return os.path.join(self.project_folder, self.name)

    def prepare_graph(self):
        self.graph_builder.build()

    # when update, the files are re-sync with the template folder
    def create_vivado_project(self, update=False):
        # vitis panic when containing upper letter
        assert self.name.lower() == self.name
        if not os.path.exists(self.project_folder):
            os.mkdir(self.project_folder)
        run_for_files = [
            self.hardware_proj_dir(),
            os.path.join(
                self.hardware_proj_dir(), f"{self.name}_verilog", "MicroBlossomBus.v"
            ),
        ]
        run = any([not os.path.exists(filename) for filename in run_for_files])
        parameters = ["--name", self.name]
        parameters += ["--path", self.project_folder]
        parameters += ["--clock-frequency", f"{self.clock_frequency}"]
        parameters += ["--clock-divide-by", f"{self.clock_divide_by}"]
        parameters += ["--graph", self.graph_builder.graph_file_path()]
        parameters += ["--broadcast-delay", f"{self.broadcast_delay}"]
        parameters += ["--convergecast-delay", f"{self.convergecast_delay}"]
        parameters += ["--context-depth", f"{self.context_depth}"]
        if not self.hard_code_weights:
            parameters += ["--dynamic-weights"]
        if not self.support_add_defect_vertex:
            parameters += ["--no-add-defect-vertex"]
        if self.support_offloading:
            parameters += ["--support-offloading"]
        if self.support_layer_fusion:
            parameters += ["--support-layer-fusion"]
        inject_registers = self.inject_registers
        if isinstance(inject_registers, str):
            inject_registers = [e for e in self.inject_registers.split(",") if e != ""]
        parameters += ["--inject-registers"] + inject_registers
        if self.overwrite:
            parameters += ["--overwrite"]
        self.project_builder = MicroBlossomProjectBuilder.from_args(
            parameters, run=run, update=update
        )

    def build_rust_binary(self, main: str = "hello_world"):
        make_env = os.environ.copy()
        make_env["EMBEDDED_BLOSSOM_MAIN"] = main
        process = subprocess.Popen(
            ["make", "Xilinx"],
            universal_newlines=True,
            stdout=sys.stdout,
            stderr=sys.stderr,
            cwd=embedded_dir,
            env=make_env,
        )
        process.wait()
        assert process.returncode == 0, "compile error"

    def has_xsa(self) -> bool:
        xsa_path = os.path.join(self.hardware_proj_dir(), f"{self.name}.xsa")
        return os.path.exists(xsa_path)

    def build_vivado_project(self, force_recompile_binary: bool = False):
        log_file_path = os.path.join(self.hardware_proj_dir(), "build.log")
        frequency = self.clock_frequency
        print(f"building frequency={frequency}, log output to {log_file_path}")
        if not self.has_xsa() or force_recompile_binary:
            with open(log_file_path, "a") as log:
                process = subprocess.Popen(
                    ["make"],
                    universal_newlines=True,
                    stdout=log.fileno(),
                    stderr=log.fileno(),
                    cwd=self.hardware_proj_dir(),
                )
                process.wait()
                assert process.returncode == 0, "synthesis error"

    # check timing reports to make sure there are no negative slacks
    def timing_sanity_check_failed(self) -> bool:
        print("start timing sanity check")
        vivado = VivadoProject(self.hardware_proj_dir())
        wns = vivado.routed_timing_summery().clk_pl_0_wns
        frequency = vivado.frequency()
        period = 1e-6 / frequency
        new_period = period - wns * 1e-9
        new_frequency = 1 / new_period / 1e6
        if wns < 0:
            # negative slack exists, need to lower the clock frequency
            print(f"[failed] frequency={frequency}MHz clock frequency too high")
            print(f"wns: {wns}ns, should lower the frequency to {new_frequency}MHz")
            return True
        else:
            print(f"[passed] frequency={frequency}MHz satisfies the timing constraint")
            print(f"    thought it could potentially run at {new_frequency}MHz")
            return False

    def run_application(self, silent: bool = True) -> str:
        log_file_path = os.path.join(self.hardware_proj_dir(), "make.log")
        print(f"testing, log output to {log_file_path}")
        with open(log_file_path, "a", encoding="utf8") as log:
            log.write(
                f"[host_event] [make run_a72 start] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            )
            tty_output, command_output = get_ttyoutput(
                command=["make", "run_a72"], cwd=self.hardware_proj_dir(), silent=silent
            )
            log.write(
                f"[host_event] [make run_a72 finish] {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            )
            log.write(f"[host_event] [tty_output]\n")
            log.write(tty_output + "\n")
            log.write(f"[host_event] [command_output]\n")
            log.write(command_output + "\n")
            return tty_output

    # this function assumes the bottleneck is the fast clock domain (self.frequency)
    # return current frequency if timing passed; otherwise return a maximum frequency that is achievable
    def next_maximum_frequency(self) -> int | None:
        vivado = VivadoProject(self.hardware_proj_dir())
        wns = vivado.routed_timing_summery().clk_pl_0_wns
        frequency = vivado.frequency()
        assert frequency == self.clock_frequency
        period = 1e-6 / frequency
        new_period = period - wns * 1e-9
        new_frequency = math.floor(1 / new_period / 1e6)
        if wns < 0:
            print(f"[failed] frequency={frequency}MHz clock frequency too high")
            print(f"wns: {wns}ns, should lower the frequency to {new_frequency}MHz")
            return new_frequency
        else:
            print(f"[passed] frequency={frequency}MHz satisfies the timing constraint")
            print(f"    thought it could potentially run at {new_frequency}MHz")
            return None

    # this function assumes the bottleneck is the slow clock domain (self.frequency / self.clock_divide_by)
    # return current value if timing passed; otherwise return a minimum clock_divide_by that is achievable
    def next_minimum_clock_divide_by(self) -> float:
        vivado = VivadoProject(self.hardware_proj_dir())
        wns = vivado.routed_timing_summery().clk_pl_0_wns
        frequency = vivado.frequency()
        assert frequency == self.clock_frequency
        if wns < 0:
            print(
                f"frequency={frequency}MHz, clock_divide_by={self.clock_divide_by} is not achievable"
            )
            period = 1e-6 / frequency
            slow_period = period * self.clock_divide_by
            new_slow_period = slow_period - wns * 1e-9
            new_clock_divide_by = new_slow_period / period
            print(f"wns: {wns}ns, clock_divide_by lower to {new_clock_divide_by}")
            return new_clock_divide_by
        else:
            return self.clock_divide_by

    def build_embedded_binary(self, make_env: dict | None):
        if make_env is None:
            make_env = os.environ.copy()
        if "EMBEDDED_BLOSSOM_MAIN" not in make_env:
            print("[warning] no EMBEDDED_BLOSSOM_MAIN, default to hello_world")
            make_env["EMBEDDED_BLOSSOM_MAIN"] = "hello_world"
        process = subprocess.Popen(
            ["make", "Xilinx"],
            universal_newlines=True,
            stdout=sys.stdout,
            stderr=sys.stderr,
            cwd=embedded_dir,
            env=make_env,
        )
        process.wait()
        assert process.returncode == 0, "compile error"

    def build(self):
        self.prepare_graph()
        self.create_vivado_project()
        self.build_rust_binary()
        self.build_vivado_project()


class HeuristicFrequencyCircuitLevel:
    """
    Heuristic from benchmark/hardware/frequency_optimization/circuit_level_offloading_layer_fusion
    """

    @staticmethod
    def of(d: int) -> int:
        assert d % 2 == 1
        assert d >= 3
        if d == 3:
            return 180
        if d == 5:
            return 141
        cycle = 3.69e-3 * (d**3) + 8.16
        return math.ceil(1000 / cycle)
