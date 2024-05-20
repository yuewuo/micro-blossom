import os
import sys
import subprocess
import math

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

interval = 10  # how many frequencies between 100MHz and 200MHz in the log scale
frequency_of = lambda idx: math.floor(100 * (2 ** (idx / interval)))
f_vec = [frequency_of(i) for i in range(interval * 5) if frequency_of(i) <= 500]
print(f_vec)


def hardware_proj_name(frequency) -> str:
    return f"f_{frequency}"


def hardware_proj_dir(frequency) -> str:
    return os.path.join(hardware_dir, hardware_proj_name(frequency))


def main():
    pass


if __name__ == "__main__":
    main()
