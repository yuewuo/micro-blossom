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

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
this_dir = os.path.dirname(os.path.abspath(__file__))
hardware_dir = os.path.join(this_dir, "hardware")


d_vec = [3, 5, 7, 9, 11, 13, 15, 17]
p_vec = [0.02, 0.01, 0.005, 0.002, 0.001]


def total_rounds(d, p):
    return 1000


def hardware_proj_name(d):
    return f"d_{d}"


def hardware_proj_dir(d):
    return os.path.join(hardware_dir, hardware_proj_name(d))


def main():
    pass


if __name__ == "__main__":
    main()
