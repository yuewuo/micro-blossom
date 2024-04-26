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

d_vec = [3, 5, 7, 9]
max_injections = 4
f_vec = [100, 95, 76, 66]
p = 0.001


def total_rounds(d, p):
    return 1000


def hardware_proj_name(d: int, inj: int):
    return f"d_{d}_inj_{inj}"


def hardware_proj_dir(d: int, inj: int):
    return os.path.join(hardware_dir, hardware_proj_name(d, inj))


def main():
    pass


if __name__ == "__main__":
    main()
