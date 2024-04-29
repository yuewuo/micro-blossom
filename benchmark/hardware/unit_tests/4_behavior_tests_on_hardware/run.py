import os
import sys
import subprocess
import shutil
from datetime import datetime
import importlib.util

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
hardware_dir = os.path.join(this_dir, "hardware")
run_dir = os.path.join(this_dir, "run")
behavior_tests_path = os.path.join(
    git_root_dir, "benchmark", "behavior", "tests", "run.py"
)

spec = importlib.util.spec_from_file_location("behavior_tests", behavior_tests_path)
behavior_tests_module = importlib.util.module_from_spec(spec)
sys.modules["behavior_tests"] = behavior_tests_module
spec.loader.exec_module(behavior_tests_module)
from behavior_tests import *

frequency = 50


def main():
    compile_code_if_necessary()


if __name__ == "__main__":
    main()
