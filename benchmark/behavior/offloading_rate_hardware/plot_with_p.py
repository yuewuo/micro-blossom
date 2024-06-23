import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
from hardware.decoding_speed.circuit_level_common import *
from run_with_p import *

this_dir = os.path.dirname(os.path.abspath(__file__))

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
        for d in d_vec:
            with open(
                os.path.join(this_dir, f"{name}_d{d}.txt"), "r", encoding="utf8"
            ) as f:
                p_vec = []
                offload_rate_vec = []
                for line in f.readlines():
                    line = line.strip("\r\n ")
                    if line == "" or line.startswith("#"):
                        continue
                    spt = line.split(" ")
                    assert len(spt) == 5
                    p_vec.append(float(spt[0]))
                    offload_rate_vec.append(float(spt[3]))
                plt.loglog(
                    p_vec, [1 - e for e in offload_rate_vec], "o-", label=f"d = {d}"
                )
        plt.xlim(min(p_vec) * 0.8, max(p_vec) * 1.2)
        plt.ylabel("1 - coverage")
        plt.gca().invert_yaxis()
        plt.xlabel("physical error rate $p$")
        plt.legend(loc="upper right")
        plt.savefig(os.path.join(this_dir, f"{name}_with_p.pdf"))
