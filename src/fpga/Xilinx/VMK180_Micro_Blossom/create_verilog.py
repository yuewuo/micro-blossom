"""
Create the verilog file from Scala in a certain folder relative to this script
"""

import os
import sys
import subprocess
from common import *

if len(sys.argv) != 2:
    print("Usage: <dual_config_filepath>")
    print("Please try again.")
    exit(1)
dual_config_filepath = sys.argv[1]

script_dir = os.path.dirname(os.path.abspath(__file__))
project_path = os.path.abspath(os.path.join(script_dir, "..", "..", "..", ".."))
target_path = os.path.join(script_dir, f"{name}_verilog")

assert os.path.exists(os.path.join(project_path, "build.sbt")), "wrong project path"

if not os.path.exists(target_path):
    os.makedirs(target_path)

# call sbt to generate the verilog at `target_path`
subprocess.Popen(
    f'sbt "runMain microblossom.MicroBlossomBusGenerator --graph {dual_config_filepath} --output-dir {target_path}"',
    shell=True,
    cwd=project_path,
).wait()
