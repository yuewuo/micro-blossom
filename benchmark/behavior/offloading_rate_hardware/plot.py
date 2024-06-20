import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
from hardware.decoding_speed.circuit_level_common import *
from run import *

this_dir = os.path.dirname(os.path.abspath(__file__))

p_vec = [0.0001, 0.0002, 0.0005, 0.001, 0.002, 0.005, 0.01]

if __name__ == "__main__":
    for name in ["pre_match", "layer_fusion"]:
        plt.cla()
        # draw dashed lines for easier grab of data
        i_vec = list(range(2, 11))
        for i in i_vec:
            coverage = i / 10
            plt.plot(
                [min(d_vec) - 1, max(d_vec) + 1],
                [coverage, coverage],
                ":",
                color="grey",
            )
        # draw data
        for p in reversed(p_vec):
            with open(
                os.path.join(this_dir, f"{name}_p{p}.txt"), "r", encoding="utf8"
            ) as f:
                d_vec = []
                offload_rate_vec = []
                for line in f.readlines():
                    line = line.strip("\r\n ")
                    if line == "" or line.startswith("#"):
                        continue
                    spt = line.split(" ")
                    assert len(spt) == 5
                    d_vec.append(int(spt[0]))
                    offload_rate_vec.append(float(spt[3]))
                plt.plot(d_vec, offload_rate_vec, "o-", label=f"p = {p}")
        plt.xlim(min(d_vec) - 1, max(d_vec) + 1)
        plt.xticks(d_vec, d_vec)
        plt.ylim(min(i_vec) / 10 - 0.02, max(i_vec) / 10 + 0.02)
        plt.yticks(
            [i / 10 for i in i_vec],
            [f"{i}0%" if i != 0 else "0%" for i in i_vec],
        )
        plt.ylabel("pre-matching coverage")
        plt.xlabel("code distance $d$")
        plt.legend(loc="lower right")
        plt.savefig(os.path.join(this_dir, f"{name}.pdf"))
