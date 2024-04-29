import os
import sys
import subprocess
import shutil
from datetime import datetime

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

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
    from get_ttyoutput import get_ttyoutput
this_dir = os.path.dirname(os.path.abspath(__file__))
run_dir = os.path.join(this_dir, "run")
graph_dir = os.path.join(git_root_dir, "resources", "graphs")

default_graph = os.path.join(graph_dir, "example_code_capacity_d3.json")

test_main = "test_micro_blossom"

variants = [
    # default
    {},
    # 4x code type
    {"graph": os.path.join(graph_dir, "example_code_capacity_rotated_d3.json")},
    {"graph": os.path.join(graph_dir, "example_code_capacity_planar_d3.json")},
    {"graph": os.path.join(graph_dir, "example_phenomenological_rotated_d3.json")},
    {"graph": os.path.join(graph_dir, "example_circuit_level_d3.json")},
    # 2x broadcast delay
    {"broadcast_delay": 2},
    {"broadcast_delay": 3},
    # 2x convergecast delay
    {"convergecast_delay": 2},
    {"convergecast_delay": 3},
    # 3x clock divided by
    {"clock_divide_by": 2},
    {"clock_divide_by": 3},
    {"clock_divide_by": 4},
    # 5x context depth
    {"context_depth": 2},
    {"context_depth": 4},
    {"context_depth": 8},
    {"context_depth": 16},
    {"context_depth": 32},
    # 12x inject registers
    {"inject_registers": "offload"},
    {"inject_registers": "offload2"},
    {"inject_registers": "offload3"},
    {"inject_registers": "offload4"},
    {"inject_registers": "execute"},
    {"inject_registers": "execute2"},
    {"inject_registers": "execute3"},
    {"inject_registers": "update"},
    {"inject_registers": "update2"},
    {"inject_registers": "update3"},
    {"inject_registers": "offload4,update3"},
    {"inject_registers": "offload3,execute2,update"},
    # 5x (clock divided by, inject registers)
    {"clock_divide_by": 2, "inject_registers": "execute2"},
    {"clock_divide_by": 2, "inject_registers": "offload4,update3"},
    {"clock_divide_by": 2, "inject_registers": "offload3,execute2,update"},
    {"clock_divide_by": 3, "inject_registers": "execute2"},
    {"clock_divide_by": 3, "inject_registers": "offload4,update3"},
    # 4x (clock divided by, context depth)
    {"clock_divide_by": 2, "context_depth": 2},
    {"clock_divide_by": 2, "context_depth": 4},
    {"clock_divide_by": 3, "context_depth": 2},
    {"clock_divide_by": 3, "context_depth": 4},
    # 6x (clock divided by, broadcast delay, convergecast delay)
    {"clock_divide_by": 2, "broadcast_delay": 2, "convergecast_delay": 1},
    {"clock_divide_by": 2, "broadcast_delay": 1, "convergecast_delay": 2},
    {"clock_divide_by": 2, "broadcast_delay": 2, "convergecast_delay": 2},
    {"clock_divide_by": 3, "broadcast_delay": 2, "convergecast_delay": 1},
    {"clock_divide_by": 3, "broadcast_delay": 1, "convergecast_delay": 2},
    {"clock_divide_by": 3, "broadcast_delay": 2, "convergecast_delay": 2},
    # 2x bus interfaces
    {"bus_type": "Axi4"},  # Axi4
    {"use_32_bus": True},  # AxiLite4Bus32
    # 4x (bus interfaces, clock divided by)
    {"bus_type": "Axi4", "clock_divide_by": 2},
    {"bus_type": "Axi4", "clock_divide_by": 3},
    {"use_32_bus": True, "clock_divide_by": 2},
    {"use_32_bus": True, "clock_divide_by": 3},
    # 1x support offloading
    {"support_offloading": True},
    # 2x (support offloading, clock divided by)
    {"support_offloading": True, "clock_divide_by": 2},
    {"support_offloading": True, "clock_divide_by": 3},
    # 2x (support offloading, broadcast delay, clock divided by)
    {"support_offloading": True, "broadcast_delay": 2, "clock_divide_by": 2},
    {"support_offloading": True, "broadcast_delay": 2, "clock_divide_by": 3},
]


def main():
    compile_code_if_necessary()

    if not os.path.exists(run_dir):
        os.mkdir(run_dir)

    print(f"There are {len(variants)} variants...")

    for var_idx, variant in enumerate(variants):
        config = {"graph": default_graph, **variant}

        name = ""
        for key in config:
            value = config[key]
            if key == "graph":
                if value != default_graph:
                    name += "-graph_" + os.path.basename(config["graph"]).split(".")[0]
            else:
                name += f"_{key}_{value}".replace(",", "-")

        if name == "":
            name = "default"
        else:
            name = name[1:]  # remove leading _

        filename = f"{name}.log"
        # print(f"generating {filename}")

        left, virtual, weight = find_edge_0(config["graph"])
        config["EDGE_0_LEFT"] = left
        config["EDGE_0_VIRTUAL"] = virtual
        config["EDGE_0_WEIGHT"] = weight

        config["EMBEDDED_BLOSSOM_MAIN"] = test_main
        with open(os.path.join(run_dir, filename), "w") as log:
            run_env = os.environ.copy()
            default_command = "cargo run --release --bin embedded_simulator --"
            # for running single failed case, printed to the head of the log file
            runnable_command = default_command
            for key in config:
                if key == "graph":
                    continue
                value = config[key]
                run_env[key.upper()] = str(value)
                runnable_command = f"{key.upper()}={str(value)} " + runnable_command
            runnable_command += " " + config["graph"]
            log.write(f"# {runnable_command}\n\n")
            log.flush()
            process = subprocess.Popen(
                default_command.split(" ") + [config["graph"]],
                universal_newlines=True,
                stdout=log.fileno(),
                stderr=log.fileno(),
                cwd=rust_dir,
                env=run_env,
            )
            process.wait()
            succeeded = process.returncode == 0
            print(f"- [{'x' if succeeded else ' '}] {var_idx}. {variant}")


# find the edge 0 for testing in the graph file, see `src/cpu/embedded/src/mains/test_micro_blossom.rs`
def find_edge_0(graph_filepath):
    graph = SingleGraph.from_file(graph_filepath)
    for edge_index in range(len(graph.weighted_edges)):
        edge = graph.weighted_edges[edge_index]
        for left, right in [(edge.l, edge.r), (edge.r, edge.l)]:
            if right in graph.virtual_vertices and left not in graph.virtual_vertices:
                return left, right, edge.w


if __name__ == "__main__":
    main()
