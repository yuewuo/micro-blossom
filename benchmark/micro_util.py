import json
import subprocess
import os
import sys
import tempfile
import math
import scipy
from dataclasses import dataclass
from dataclasses_json import dataclass_json
from typing import Optional


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
rust_dir = os.path.join(git_root_dir, "src", "cpu", "blossom")
embedded_dir = os.path.join(git_root_dir, "src", "cpu", "embedded")
benchmark_dir = os.path.join(git_root_dir, "benchmark")

# please put the fusion-blossom folder next to this project folder
fusion_dir = os.path.join(git_root_dir, "..", "fusion-blossom")
if not os.path.exists(fusion_dir):
    print("please put the fusion-blossom folder next to this project folder")
    exit(1)
fusion_benchmark_dir = os.path.join(fusion_dir, "benchmark")
if True:
    sys.path.insert(0, fusion_benchmark_dir)
    from util import compile_code_if_necessary as fusion_compile_code_if_necessary
    from util import fusion_blossom_qecp_generate_command, fusion_blossom_command
    from util import run_command_get_stdout as fusio_run_command_get_stdout

MICRO_BLOSSOM_COMPILATION_DONE = False
if (
    "MANUALLY_COMPILE_QEC" in os.environ
    and os.environ["MANUALLY_COMPILE_QEC"] == "TRUE"
):
    MICRO_BLOSSOM_COMPILATION_DONE = True

SCALA_MICRO_BLOSSOM_COMPILATION_DONE = False
if (
    "MANUALLY_COMPILE_QEC" in os.environ
    and os.environ["MANUALLY_COMPILE_QEC"] == "TRUE"
):
    SCALA_MICRO_BLOSSOM_COMPILATION_DONE = True


class Profile:
    """
    read profile given filename; if provided `skip_begin_profiles`, then it will skip such number of profiles in the beginning,
    by default to 5 because usually the first few profiles are not stable yet
    """

    def __init__(self, filename, skip_begin_profiles=20):
        assert isinstance(filename, str)
        self.partition_config = None
        self.entries = []
        skipped = 0
        with open(filename, "r", encoding="utf8") as f:
            for line_idx, line in enumerate(f.readlines()):
                line = line.strip("\r\n ")
                if line == "":
                    break
                value = json.loads(line)
                if line_idx == 0:
                    self.partition_config = PartitionConfig.from_json(value)
                elif line_idx == 1:
                    self.benchmark_config = value
                else:
                    if skipped < skip_begin_profiles:
                        skipped += 1
                    else:
                        self.entries.append(value)

    def __repr__(self):
        return f"Profile {{ partition_config: {self.partition_config}, entries: [...{len(self.entries)}] }}"

    def sum_decoding_time(self):
        decoding_time = 0
        for entry in self.entries:
            decoding_time += entry["events"]["decoded"]
        return decoding_time

    def decoding_time_relative_dev(self):
        dev_sum = 0
        avr_decoding_time = self.average_decoding_time()
        for entry in self.entries:
            dev_sum += (entry["events"]["decoded"] - avr_decoding_time) ** 2
        return math.sqrt(dev_sum / len(self.entries)) / avr_decoding_time

    def average_decoding_time(self):
        return self.sum_decoding_time() / len(self.entries)

    def sum_defect_num(self):
        defect_num = 0
        for entry in self.entries:
            defect_num += entry["defect_num"]
        return defect_num

    def average_decoding_time_per_defect(self):
        return self.sum_decoding_time() / self.sum_defect_num()

    def sum_computation_cpu_seconds(self):
        total_computation_cpu_seconds = 0
        for entry in self.entries:
            computation_cpu_seconds = 0
            for event_time in entry["solver_profile"]["primal"]["event_time_vec"]:
                computation_cpu_seconds += event_time["end"] - event_time["start"]
            total_computation_cpu_seconds += computation_cpu_seconds
        return total_computation_cpu_seconds

    def average_computation_cpu_seconds(self):
        return self.sum_computation_cpu_seconds() / len(self.entries)

    def sum_job_time(self, unit_index):
        total_job_time = 0
        for entry in self.entries:
            event_time = entry["solver_profile"]["primal"]["event_time_vec"][unit_index]
            total_job_time += event_time["end"] - event_time["start"]
        return total_job_time

    def average_job_time(self, unit_index):
        return self.sum_job_time(unit_index) / len(self.entries)

    def sum_offloaded(self):
        offloaded = 0
        for entry in self.entries:
            offloaded += entry["solver_profile"]["primal"]["offloaded"]
        return offloaded


class VertexRange:
    def __init__(self, start, end):
        self.range = (start, end)

    def __repr__(self):
        return f"[{self.range[0]}, {self.range[1]}]"

    def length(self):
        return self.range[1] - self.range[0]


class PartitionConfig:
    def __init__(self, vertex_num):
        self.vertex_num = vertex_num
        self.partitions = [VertexRange(0, vertex_num)]
        self.fusions = []
        self.parents = [None]

    def __repr__(self):
        return f"PartitionConfig {{ vertex_num: {self.vertex_num}, partitions: {self.partitions}, fusions: {self.fusions} }}"

    @staticmethod
    def from_json(value):
        vertex_num = value["vertex_num"]
        config = PartitionConfig(vertex_num)
        config.partitions.clear()
        for vertex_range in value["partitions"]:
            config.partitions.append(VertexRange(vertex_range[0], vertex_range[1]))
        for pair in value["fusions"]:
            config.fusions.append((pair[0], pair[1]))
        assert len(config.partitions) == len(config.fusions) + 1
        unit_count = len(config.partitions) * 2 - 1
        # build parent references
        parents = [None] * unit_count
        for fusion_index, (left_index, right_index) in enumerate(config.fusions):
            unit_index = fusion_index + len(config.partitions)
            assert left_index < unit_index
            assert right_index < unit_index
            assert parents[left_index] is None
            assert parents[right_index] is None
            parents[left_index] = unit_index
            parents[right_index] = unit_index
        for unit_index in range(unit_count - 1):
            assert parents[unit_index] is not None
        assert parents[unit_count - 1] is None
        config.parents = parents
        return config

    def unit_depth(self, unit_index):
        depth = 0
        while self.parents[unit_index] is not None:
            unit_index = self.parents[unit_index]
            depth += 1
        return depth


def compile_code_if_necessary(additional_build_parameters=None):
    global MICRO_BLOSSOM_COMPILATION_DONE
    if MICRO_BLOSSOM_COMPILATION_DONE is False:
        build_parameters = ["cargo", "build", "--release"]
        if additional_build_parameters is not None:
            build_parameters += additional_build_parameters
        # print(build_parameters)
        process = subprocess.Popen(
            build_parameters,
            universal_newlines=True,
            stdout=sys.stdout,
            stderr=sys.stderr,
            cwd=rust_dir,
        )
        process.wait()
        assert process.returncode == 0, "compile has error"
        MICRO_BLOSSOM_COMPILATION_DONE = True
    fusion_compile_code_if_necessary(["--features", "qecp_integrate"])


def compile_scala_micro_blossom_if_necessary():
    global SCALA_MICRO_BLOSSOM_COMPILATION_DONE
    if SCALA_MICRO_BLOSSOM_COMPILATION_DONE is False:
        process = subprocess.Popen(
            ["sbt", "assembly"],
            universal_newlines=True,
            stdout=sys.stdout,
            stderr=sys.stderr,
            cwd=git_root_dir,
        )
        process.wait()
        assert process.returncode == 0, "compile has error"
        SCALA_MICRO_BLOSSOM_COMPILATION_DONE = True


def micro_blossom_command():
    micro_path = os.path.join(rust_dir, "target", "release", "micro_blossom")
    return [micro_path]


def micro_blossom_benchmark_command(
    d=None, p=None, total_rounds=None, r=None, noisy_measurements=None, n=None
):
    assert d is not None
    assert p is not None
    command = micro_blossom_command() + ["benchmark", f"{d}", f"{p}"]
    if total_rounds is not None:
        command += ["-r", f"{total_rounds}"]
    elif r is not None:
        command += ["-r", f"{r}"]
    if noisy_measurements is not None:
        command += ["-n", f"{noisy_measurements}"]
    elif n is not None:
        command += ["-n", f"{n}"]
    return command


def run_command_get_stdout(
    command, no_stdout=False, use_tmp_out=False, stderr_to_stdout=False
):
    compile_code_if_necessary()
    env = os.environ.copy()
    env["RUST_BACKTRACE"] = "full"
    stdout = subprocess.PIPE
    if use_tmp_out:
        out_file = tempfile.NamedTemporaryFile(delete=False)
        out_filename = out_file.name
        stdout = out_file
    if no_stdout:
        stdout = sys.stdout
    process = subprocess.Popen(
        command,
        universal_newlines=True,
        env=env,
        stdout=stdout,
        stderr=(stdout if stderr_to_stdout else sys.stderr),
        bufsize=100000000,
    )
    stdout, _ = process.communicate()
    if use_tmp_out:
        out_file.flush()
        out_file.close()
        with open(out_filename, "r", encoding="utf8") as f:
            stdout = f.read()
        os.remove(out_filename)
    return stdout, process.returncode


class GnuplotData:
    def __init__(self, filename):
        assert isinstance(filename, str)
        with open(filename, "r", encoding="utf8") as f:
            lines = f.readlines()
        self.titles = []
        if lines[0].startswith("<"):  # title line
            line = lines[0].strip("\r\n ")
            titles = line.split(" ")
            for title in titles:
                # assert title.startswith("<") and title.endswith(">")
                self.titles.append(title[1:-1])
            lines = lines[1:]
        self.data = []
        for line in lines:
            line = line.strip("\r\n ")
            self.data.append(line.split(" "))

    def fit(
        self,
        x_column,
        y_column,
        x_func=lambda x: float(x),
        y_func=lambda y: float(y),
        starting_row=0,
        ending_row=None,
    ):
        X = [x_func(line[x_column]) for line in self.data[starting_row:ending_row]]
        Y = [y_func(line[y_column]) for line in self.data[starting_row:ending_row]]
        slope, intercept, r, _, _ = scipy.stats.linregress(X, Y)
        return slope, intercept, r


@dataclass_json
@dataclass
class SolverInitializer:
    vertex_num: int
    weighted_edges: list[list[int]]
    virtual_vertices: list[int]


@dataclass_json
@dataclass
class SyndromePattern:
    defect_vertices: list[int]
    erasures: list[int]
    dynamic_weights: Optional[list[list[int]]] = None


@dataclass_json
@dataclass
class VertexPosition:
    i: float
    j: float
    t: float


@dataclass
class SyndromesV1:
    initializer: SolverInitializer
    positions: list[VertexPosition]
    syndromes: list[SyndromePattern]

    @staticmethod
    def from_file(filename):
        assert isinstance(filename, str)
        with open(filename, "r", encoding="utf8") as f:
            head = f.readline()
            assert head.startswith("Syndrome Pattern v1.0 ")
            # Syndrome Pattern v1.0   <initializer> <positions> <syndrome_pattern>*
            initializer_str = f.readline()
            initializer = SolverInitializer.schema().loads(initializer_str)
            positions = f.readline()
            positions = VertexPosition.schema().loads(positions, many=True)
            line = f.readline()
            line.strip("\r\n ")
            syndromes = []
            while line != "":
                syndrome_pattern = SyndromePattern.schema().loads(line)
                syndromes.append(syndrome_pattern)
                line = f.readline()
                line.strip("\r\n ")
        return SyndromesV1(initializer, positions, syndromes)


@dataclass_json
@dataclass
class Position:
    i: float
    j: float
    t: float


@dataclass_json
@dataclass
class WeightedEdge:
    l: int
    r: int
    w: int


@dataclass_json
@dataclass
class BinaryTreeNode:
    p: Optional[int] = None
    l: Optional[int] = None
    r: Optional[int] = None


@dataclass_json
@dataclass
class BinaryTree:
    nodes: list[BinaryTreeNode]


@dataclass_json
@dataclass
class DefectMatch:
    e: int


@dataclass_json
@dataclass
class VirtualMatch:
    e: int
    v: int


@dataclass_json
@dataclass
class Offloading:
    dm: Optional[DefectMatch] = None
    vm: Optional[VirtualMatch] = None


@dataclass_json
@dataclass
class LayerFusion:
    num_layers: int
    layers: list[list[int]]
    vertex_layer_id: dict[int, int]
    fusion_edges: dict[int, int]
    unique_tight_conditions: dict[int, list[int]]


@dataclass_json
@dataclass
class SingleGraph:
    positions: list[Position]
    vertex_num: int
    weighted_edges: list[WeightedEdge]
    virtual_vertices: list[int]
    vertex_binary_tree: BinaryTree
    edge_binary_tree: BinaryTree
    vertex_edge_binary_tree: BinaryTree
    vertex_max_growth: list[int]
    offloading: list[Offloading]
    layer_fusion: Optional[LayerFusion]

    @staticmethod
    def from_file(filename):
        assert isinstance(filename, str)
        with open(filename, "r", encoding="utf8") as f:
            value = f.read()
        return SingleGraph.schema().loads(value)

    def effective_offloader_num(
        self, support_offloading: bool, support_layer_fusion: bool
    ) -> int:
        if not support_offloading:
            return 0
        if support_layer_fusion and self.layer_fusion is not None:
            return len(self.offloading) + sum(
                len(layer) for layer in self.layer_fusion.layers
            )
        return len(self.offloading)
