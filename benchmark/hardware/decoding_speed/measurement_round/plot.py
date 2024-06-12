import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
from hardware.decoding_speed.circuit_level_common import *

this_dir = os.path.dirname(os.path.abspath(__file__))

if __name__ == "__main__":
    name = os.path.basename(this_dir)
    plt.cla()
    for label in ["batch", "fusion"]:
        with open(os.path.join(this_dir, f"{label}.txt"), "r", encoding="utf8") as f:
            rounds_vec = []
            latency_vec = []
            for line in f.readlines():
                line = line.strip("\r\n ")
                if line == "" or line.startswith("#"):
                    continue
                spt = line.split(" ")
                assert len(spt) == 4
                rounds_vec.append(int(spt[1]))
                latency_vec.append(float(spt[3]))
            plt.plot(rounds_vec, latency_vec, "o-", label=label)
    xticks = list(range(2, 20))
    plt.xticks(xticks, xticks)
    plt.ylabel("decoding latency")
    plt.xlabel("number of measurement rounds")
    plt.legend()
    plt.savefig(os.path.join(this_dir, f"{name}.pdf"))
