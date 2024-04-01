import argparse
import shlex
import subprocess
import sys
import os
import re


git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(
    __file__)), shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")

SCALA_MICRO_BLOSSOM_COMPILATION_DONE = False


def run_verilog_generator(parameters):
    # first compile the Scala library
    global SCALA_MICRO_BLOSSOM_COMPILATION_DONE
    if not SCALA_MICRO_BLOSSOM_COMPILATION_DONE is False:
        process = subprocess.Popen(["sbt", "assembly"], universal_newlines=True,
                                   stdout=sys.stdout, stderr=sys.stderr, cwd=git_root_dir)
        process.wait()
        assert process.returncode == 0, "compile has error"
        SCALA_MICRO_BLOSSOM_COMPILATION_DONE = True
    # then run the generator
    command = ["java", "-cp",
               os.path.join(git_root_dir, "target/scala-2.12/microblossom.jar"), "microblossom.MicroBlossomGenerator"] + parameters
    process = subprocess.Popen(command, universal_newlines=True,
                               stdout=sys.stdout, stderr=sys.stderr, cwd=git_root_dir)
    process.wait()
    assert process.returncode == 0, "error when running the generator"


def main(args=None):
    parser = argparse.ArgumentParser(description='Build Micro Blossom')
    parser.add_argument('-n', '--name', required=True,
                        help='the name of the Vivado project')
    parser.add_argument('-p', '--path', default=".",
                        help='folder of the project, a subfolder will be created with the project name')
    parser.add_argument('-b', '--board', default="VMK180",
                        help='FPGA board, e.g., VMK180')
    parser.add_argument('-c', '--clock-frequency', default="100e6",
                        help='clock frequency in Hz')
    parser.add_argument('-g', '--graph', required=True,
                        help='the graph passed as the argument --graph in MicroBlossomGenerator; it also searches in /resources/graphs/')
    args, parameters = parser.parse_known_args(args=args)

    print("Configurations:")
    board = args.board
    print(f"board: {board}")
    clock_frequency = float(args.clock_frequency)
    print(f"clock frequency: {clock_frequency}")
    path = args.path
    print(f"path: {path}")
    name = args.name
    print(f"project name: {name}")
    assert re.match(r'^[a-zA-Z0-9_-]+$', name), f"invalid project name {name}"
    project_folder = os.path.join(path, name)
    if not os.path.exists(project_folder):
        os.makedirs(project_folder)
    graph = args.graph
    if not os.path.exists(graph):
        graph = os.path.join(git_root_dir, "resources", "graphs", graph)
    assert os.path.exists(graph)

    verilog_path = os.path.abspath(
        os.path.join(project_folder, f"{name}_verilog"))
    if not os.path.exists(verilog_path):
        os.makedirs(verilog_path)
    parameters += ["--output-dir", verilog_path, "--graph", graph]
    print("the following parameters will be passed to the Scala main function (microblossom.MicroBlossomGenerator):")
    print(f"    {' '.join([shlex.quote(para) for para in parameters])}")

    run_verilog_generator(parameters)
    print("Verilog Generated")


if __name__ == "__main__":
    main()
