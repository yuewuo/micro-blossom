import argparse
import shlex
import subprocess
import sys
import os
import re
import shutil
import git


git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
template_dir = os.path.join(
    git_root_dir, "src", "fpga", "Xilinx", "VMK180_Micro_Blossom"
)
embedded_dir = os.path.join(git_root_dir, "src", "cpu", "embedded")

SCALA_MICRO_BLOSSOM_COMPILATION_DONE = False


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


def run_verilog_generator(parameters):
    # first compile the Scala library
    compile_scala_micro_blossom_if_necessary()
    # then run the generator
    command = [
        "java",
        "-Xmx32G",
        "-cp",
        os.path.join(git_root_dir, "target/scala-2.12/microblossom.jar"),
        "microblossom.MicroBlossomBusGenerator",
    ] + parameters
    process = subprocess.Popen(
        command,
        universal_newlines=True,
        stdout=sys.stdout,
        stderr=sys.stderr,
        cwd=git_root_dir,
    )
    process.wait()
    assert process.returncode == 0, "error when running the generator"


def main(args=None):
    parser = argparse.ArgumentParser(description="Build Micro Blossom")
    parser.add_argument(
        "-n", "--name", required=True, help="the name of the Vivado project"
    )
    parser.add_argument(
        "-p",
        "--path",
        default=".",
        help="folder of the project, a subfolder will be created with the project name",
    )
    parser.add_argument(
        "-b", "--board", default="VMK180", help="FPGA board, e.g., VMK180"
    )
    parser.add_argument(
        "-c", "--clock-frequency", default="200", help="clock frequency in MHz"
    )
    parser.add_argument("-d", "--clock-divide-by", default="2", help="clock divide by")
    parser.add_argument(
        "-g",
        "--graph",
        required=True,
        help="the graph passed as the argument --graph in MicroBlossomBusGenerator; it also searches in /resources/graphs/",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="regenerate the verilog and copy the files from the template directory",
    )
    args, parameters = parser.parse_known_args(args=args)

    print("Configurations:")
    board = args.board
    print(f"board: {board}")
    clock_frequency = float(args.clock_frequency)
    clock_divide_by = int(args.clock_divide_by)
    print(f"clock frequency: {clock_frequency}MHz")
    print(f"clock divide by: {clock_divide_by}")
    path = args.path
    print(f"path: {path}")
    name = args.name
    print(f"project name: {name}")
    assert re.match(r"^[a-zA-Z0-9_-]+$", name), f"invalid project name {name}"
    project_dir = os.path.join(path, name)
    if os.path.exists(project_dir) and not args.overwrite:
        print(
            f"folder {project_dir} already exists, please use `--overwrite` option to overwrite the existing files"
        )
        exit(1)
    if not os.path.exists(project_dir):
        os.makedirs(project_dir)
    graph = args.graph
    if not os.path.exists(graph):
        graph = os.path.join(git_root_dir, "resources", "graphs", graph)
    assert os.path.exists(graph)

    verilog_path = os.path.abspath(os.path.join(project_dir, f"{name}_verilog"))
    if not os.path.exists(verilog_path):
        os.makedirs(verilog_path)
    parameters += [
        "--output-dir",
        verilog_path,
        "--graph",
        graph,
        "--clock-divide-by",
        f"{clock_divide_by}",
    ]
    print(
        "the following parameters will be passed to the Scala main function (microblossom.MicroBlossomBusGenerator):"
    )
    print(f"    {' '.join([shlex.quote(para) for para in parameters])}")

    print("Generating Verilog")
    run_verilog_generator(parameters)

    print("Copying the project files")
    # common.py
    with open(os.path.join(template_dir, "common.py"), "r", encoding="utf8") as f:
        common_py = f.read()
        common_py = checked_replace(
            common_py, 'name = "vmk180_micro_blossom"', f'name = "{name}"'
        )
        common_py = checked_replace(
            common_py,
            'rust_project = "../../../cpu/embedded"',
            f'rust_project = "{embedded_dir}"',
        )
    with open(os.path.join(project_dir, "common.py"), "w", encoding="utf8") as f:
        f.write(common_py)
    # Makefile
    with open(os.path.join(template_dir, "Makefile"), "r", encoding="utf8") as f:
        makefile = f.read()
        makefile = checked_replace(
            makefile, "NAME = vmk180_micro_blossom", f"NAME = {name}"
        )
        makefile = checked_replace(
            makefile, "CLOCK_FREQUENCY ?= 200", f"CLOCK_FREQUENCY ?= {clock_frequency}"
        )
        makefile = checked_replace(
            makefile, "CLOCK_DIVIDE_BY ?= 2", f"CLOCK_DIVIDE_BY ?= {clock_divide_by}"
        )
    with open(os.path.join(project_dir, "Makefile"), "w", encoding="utf8") as f:
        f.write(makefile)
    # create_vitis.py
    shutil.copy2(
        os.path.join(template_dir, "create_vitis.py"),
        os.path.join(project_dir, "create_vitis.py"),
    )
    # create_vivado.tcl and reimpl_vivado.tcl
    for tcl_filename in ["create_vivado.tcl", "reimpl_vivado.tcl"]:
        with open(os.path.join(template_dir, tcl_filename), "r", encoding="utf8") as f:
            vivado_tcl = f.read()
            vivado_tcl = checked_replace(
                vivado_tcl, "set name vmk180_micro_blossom", f"set name {name}"
            )
        with open(os.path.join(project_dir, tcl_filename), "w", encoding="utf8") as f:
            f.write(vivado_tcl)
    # run_xsdb.tcl
    with open(os.path.join(template_dir, "run_xsdb.tcl"), "r", encoding="utf8") as f:
        run_xsdb_tcl = f.read()
        run_xsdb_tcl = checked_replace(
            run_xsdb_tcl, "set name vmk180_micro_blossom", f"set name {name}"
        )
    with open(os.path.join(project_dir, "run_xsdb.tcl"), "w", encoding="utf8") as f:
        f.write(run_xsdb_tcl)
    # src/*.c
    if not os.path.exists(os.path.join(project_dir, "src")):
        os.makedirs(os.path.join(project_dir, "src"))
    shutil.copy2(
        os.path.join(template_dir, "src", "main.c"),
        os.path.join(project_dir, "src", "main.c"),
    )
    # src/binding.c
    with open(
        os.path.join(template_dir, "src", "binding.c"), "r", encoding="utf8"
    ) as f:
        binding_c = f.read()
        binding_c = checked_replace(
            binding_c,
            "const float TIMER_FREQUENCY = 200e6; // 200MHz",
            f"const float TIMER_FREQUENCY = {clock_frequency}e6; // {clock_frequency}MHz",
        )
    with open(os.path.join(project_dir, "src", "binding.c"), "w", encoding="utf8") as f:
        f.write(binding_c)


def checked_replace(original, old, new):
    assert original.count(old) == 1, f"{old} should appear exactly once, sanity check"
    return original.replace(old, new)


if __name__ == "__main__":
    main()
