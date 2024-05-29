import os
import sys
import subprocess
import git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
if True:
    from micro_util import *

    sys.path.insert(0, fusion_benchmark_dir)
    from util import run_command_get_stdout
this_dir = os.path.dirname(os.path.abspath(__file__))
hardware_dir = os.path.join(this_dir, "hardware")


d_vec = [3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27]
p_vec = [0.4, 0.3, 0.2, 0.1, 0.05, 0.02, 0.01]


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
