import os, sys, git
import matplotlib.pyplot as plt


git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *

this_dir = os.path.dirname(os.path.abspath(__file__))

nm_vec = [2, 3, 4, 5, 9, 19]

for name in ["fusion", "batch"]:

    plt.cla()

    for nm in nm_vec:
        filename = os.path.join(this_dir, f"{name}_{nm}.txt")
        if not os.path.exists(filename):
            print(f"cannot find {filename}, skip")
            continue
        with open(filename, "r", encoding="utf8") as f:
            latency = TimeDistribution.from_line(f.read())
            x_vec, y_vec = latency.flatten()
            plt.loglog(x_vec, y_vec, ".-", label=f"nm = {nm}")

    plt.xlim(1e-7, 1e-4)
    plt.ylim(0.5, 1e8)
    plt.ylabel("Sample Count")
    plt.xlabel("Decoding Latency (s)")
    plt.legend()
    plt.savefig(os.path.join(this_dir, f"{name}.pdf"))
