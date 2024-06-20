import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
from hardware.decoding_speed.circuit_level_common import *

this_dir = os.path.dirname(os.path.abspath(__file__))

if __name__ == "__main__":
    name = os.path.basename(this_dir)
    plt.cla()
    d_vec = [3, 5, 7, 9, 11, 13, 15]
    for d in d_vec:
        with open(os.path.join(this_dir, f"d_{d}.txt"), "r", encoding="utf8") as f:
            measurement_cycle_vec = []
            latency_vec = []
            for line in f.readlines():
                line = line.strip("\r\n ")
                if line == "" or line.startswith("#"):
                    continue
                spt = line.split(" ")
                assert len(spt) == 3
                measurement_cycle_vec.append(int(spt[0]) / 1e9)
                latency_vec.append(float(spt[1]))
            plt.loglog(measurement_cycle_vec, latency_vec, "o-", label=f"d = {d}")
    plt.ylabel("decoding latency ($s$)")
    plt.xlabel("measurement cycle ($s$)")
    plt.legend()
    plt.savefig(os.path.join(this_dir, f"{name}.pdf"))
