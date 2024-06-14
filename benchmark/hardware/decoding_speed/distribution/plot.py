import os, sys, git
import matplotlib.pyplot as plt


git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *

names = ["d_9_p_0.001_batch", "d_9_p_0.001_fusion", "d_9_p_0.001_no_offloading"]

for name in names:
    with open(f"{name}.txt", "r", encoding="utf8") as f:
        latency = TimeDistribution.from_line(f.read())

        x_vec, y_vec = latency.flatten()

        plt.cla()
        plt.loglog(x_vec, y_vec, ".-")
        plt.xlim(1e-7, 1e-4)
        plt.ylim(0.5, 1e9)
        plt.ylabel("Sample Count")
        plt.xlabel("Decoding Latency (s)")
        plt.savefig(f"{name}.pdf")
