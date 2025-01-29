import os
import sys
import math
import git
from dataclasses import dataclass

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
if True:
    from micro_util import *

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
this_dir = os.path.dirname(os.path.abspath(__file__))
tmp_dir = os.path.join(this_dir, "tmp")


def main():
    for d in range(3, 22, 2):
        calculate(d=d)


@dataclass
class Result:
    vertex_num: int
    edge_num: int
    offloading_num: int
    vertex_bits: int
    edge_bits: int
    total_bits: int


def calculate(d=3, p=0.001, max_half_weight=7, verbose: bool = True) -> Result:
    compile_code_if_necessary()

    if not os.path.exists(tmp_dir):
        os.mkdir(tmp_dir)

    syndrome_file_path = os.path.join(tmp_dir, f"d_{d}.syndromes")
    if not os.path.exists(syndrome_file_path):
        command = fusion_blossom_qecp_generate_command(
            d=d, p=p, total_rounds=10, noisy_measurements=d - 1
        )
        command += ["--code-type", "rotated-planar-code"]
        command += ["--noise-model", "stim-noise-model"]
        command += [
            "--decoder",
            "fusion",
            "--decoder-config",
            f'{{"only_stab_z":true,"use_combined_probability":true,"skip_decoding":true,"max_half_weight":{max_half_weight}}}',
        ]
        command += [
            "--debug-print",
            "fusion-blossom-syndrome-file",
            "--fusion-blossom-syndrome-export-filename",
            syndrome_file_path,
        ]
        command += ["--parallel", f"0"]  # use all cores
        if verbose:
            print(command)
        stdout, returncode = run_command_get_stdout(command)
        if verbose:
            print("\n" + stdout)
        assert returncode == 0, "command fails..."

    # then generate the graph json
    graph_file_path = os.path.join(tmp_dir, f"d_{d}.json")
    if not os.path.exists(graph_file_path):
        command = micro_blossom_command() + ["parser"]
        command += [syndrome_file_path]
        command += ["--graph-file", graph_file_path]
        if verbose:
            print(command)
        stdout, returncode = run_command_get_stdout(command)
        if verbose:
            print("\n" + stdout)
        assert returncode == 0, "command fails..."

    # then cound statistics
    graph = SingleGraph.from_file(graph_file_path)
    if verbose:
        print(
            f"d = {d}: |V| = {graph.vertex_num}, |E| = {len(graph.weighted_edges)}, |P| = {len(graph.offloading)}"
        )

    # calculate storage length
    defect_bits = math.ceil(math.log2(graph.vertex_num))
    max_weight = max([e.w for e in graph.weighted_edges])
    assert max_weight == max_half_weight * 2
    grown_bits = math.ceil(math.log2(max_weight * (d - 1) // 2))
    # speed, node, root, defect, grown, virtual
    vertex_bits = 2 + (defect_bits + 1) + defect_bits + 1 + grown_bits + 1
    edge_bits = math.ceil(math.log2(max_weight))
    total_bits = vertex_bits * graph.vertex_num + edge_bits * len(graph.weighted_edges)
    if verbose:
        print(f"    bits/v = {vertex_bits}, bits/e = {edge_bits}, total = {total_bits}")

    return Result(
        vertex_num=graph.vertex_num,
        edge_num=len(graph.weighted_edges),
        offloading_num=len(graph.offloading),
        vertex_bits=vertex_bits,
        edge_bits=edge_bits,
        total_bits=total_bits,
    )


if __name__ == "__main__":
    main()
