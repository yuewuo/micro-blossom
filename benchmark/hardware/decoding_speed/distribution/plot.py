import os, sys, git, json

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *

names = [
    "d_9_p_0.001_batch",
    "d_9_p_0.001_fusion",
]

for name in names:
    with open(f"{name}.txt", "r", encoding="utf8") as f:
        latency = TimeDistribution.from_line(f.read())
        print(latency.average_latency())
