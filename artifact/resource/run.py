import git
import sys
import os
from dataclasses import dataclass
import matplotlib.pyplot as plt

this_dir = os.path.dirname(os.path.abspath(__file__))
git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))

from hardware.resource_estimate.circuit_level.run import calculate
from hardware.frequency_optimization.circuit_level_final.run import (
    main as circuit_level_final_main,
    configurations,
)


@dataclass
class Column:
    d: int
    vertex_num: int
    edge_num: int
    vertex_bits: int
    edge_bits: int
    total_bits: int
    cpu_memory_bytes: int
    clb_luts: int
    frequency: int


def main() -> list[Column]:
    print(
        "reproducing the result of Table 4 (Resource usage and maximum clock frequency) of the Micro Blossom paper"
    )

    title_print("first run the Vivado project generator")
    circuit_level_final_main()

    title_print("then retrieve the information for each code distance")
    columns = []
    for configuration in configurations:
        d = configuration.d

        # First calculate the number of vertices, edges and the number of bits
        result = calculate(d=d, verbose=False)

        # software memory usage calculation (--features compact) (index: u16, weight: i16, layer_id: u8, timestamp: u32):
        # blossom tracker: (hit_zero_events 8B + checkpoints: 8B + grow_states: 1B) * |V| = 17B * |V|
        # layer_fusion: (vertex_layer_id 1B + pending_breaks 2B) * |V| = 3B * |V|
        # nodes: (buffer 16B + first_blossom_child 2B) = 20B * (2*|V|, because nodes can be as much as 2*|V|)
        # overall cost: (17B + 3B + 40B) = 60B * |V|
        cpu_memory_bytes = 60 * result.vertex_num

        # get resource usage from Vivado project
        project = configuration.optimized_project()
        assert project.has_xsa(), "Vivado project must build successfully"
        vivado = project.get_vivado()
        report = vivado.report_impl_utilization()
        clb_luts = report.netlist_logic.clb_luts.used

        # get design frequency in MHz
        frequency = vivado.frequency()
        assert int(frequency) == frequency
        frequency = int(frequency)

        column = Column(
            d=d,
            vertex_num=result.vertex_num,
            edge_num=result.edge_num,
            vertex_bits=result.vertex_bits,
            edge_bits=result.edge_bits,
            total_bits=result.total_bits,
            cpu_memory_bytes=cpu_memory_bytes,
            clb_luts=clb_luts,
            frequency=frequency,
        )
        print(column)
        columns.append(column)

    title_print("generating table PDF")
    generate_table(columns, "resource_usage.pdf")

    return columns


def title_print(*args, **kwargs):
    print("\n\n####################################################")
    print(*args, **kwargs)
    print("####################################################\n\n")


def generate_table(columns: list[Column], filename: str):
    fig = plt.figure()
    fig.clear()
    fields = [
        ("$d$", "d", None),
        ("$|V|$", "vertex_num", None),
        ("$|E|$", "edge_num", None),
        ("CPU Mem", "cpu_memory_bytes", lambda x: f"{x/1000:.1f}kB"),
        ("vPU Mem", "vertex_bits", lambda x: f"{x}b"),
        ("ePU Mem", "edge_bits", lambda x: f"{x}b"),
        ("FPGA Mem", "total_bits", lambda x: f"{x/1000:.1f}kb"),
        ("LUTs", "clb_luts", lambda x: f"{x/1000:.1f}k"),
        ("Freq (MHz)", "frequency", None),
    ]
    cell_text = []
    for field_name, key, str_of in fields:
        if str_of is None:
            str_of = lambda x: str(x)
        row_data = [field_name]
        for column in columns:
            value = getattr(column, key)
            value_str = str_of(value)
            row_data.append(value_str)
        cell_text.append(row_data)
    the_table = plt.table(
        cellText=cell_text,
        rowLoc="right",
        loc="center",
    )
    the_table.auto_set_column_width(list(range(len(columns) + 1)))
    ax = plt.gca()
    ax.get_xaxis().set_visible(False)
    ax.get_yaxis().set_visible(False)
    plt.box(on=None)
    plt.savefig(os.path.join(this_dir, filename))


if __name__ == "__main__":
    main()
