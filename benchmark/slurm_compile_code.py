import os
import sys
import subprocess
import git

# force compile
del os.environ["MANUALLY_COMPILE_QEC"]


git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
from micro_util import *
import micro_util

fusion_benchmark_dir = os.path.join(fusion_dir, "benchmark")
sys.path.insert(0, fusion_benchmark_dir)
import util as fusion_util


def force_compile_code():
    micro_util.MICRO_BLOSSOM_COMPILATION_DONE = False
    fusion_util.FUSION_BLOSSOM_COMPILATION_DONE = False

    # micro blossom (including the fusion blossom tools)
    compile_code_if_necessary()


def force_compile_scala_micro_blossom():
    micro_util.SCALA_MICRO_BLOSSOM_COMPILATION_DONE = False

    # micro blossom scala project
    compile_scala_micro_blossom_if_necessary()


if __name__ == "__main__":
    force_compile_code()
