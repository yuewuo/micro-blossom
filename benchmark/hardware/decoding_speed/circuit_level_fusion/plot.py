import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
from hardware.decoding_speed.circuit_level_common import *

this_dir = os.path.dirname(os.path.abspath(__file__))

if __name__ == "__main__":
    plot_data(this_dir)
