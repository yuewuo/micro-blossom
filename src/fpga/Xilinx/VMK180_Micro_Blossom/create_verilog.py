"""
Create the verilog file from Scala in a certain folder relative to this script
"""

import os
import subprocess
from common import *

script_dir = os.path.dirname(os.path.abspath(__file__))
project_path = os.path.abspath(os.path.join(script_dir, "..", "..", "..", ".."))
target_path = os.path.join(script_dir, f"{name}_verilog")

assert os.path.exists(os.path.join(project_path, "build.sbt")), "wrong project path"

if not os.path.exists(target_path):
    os.makedirs(target_path)

# call sbt to generate the verilog at `target_path`
subprocess.Popen(f'sbt "runMain MicroBlossomVerilog {target_path}"', shell=True, cwd=project_path).wait()
