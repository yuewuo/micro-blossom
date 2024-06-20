import os
import matplotlib.pyplot as plt

this_dir = os.path.dirname(os.path.abspath(__file__))

names = ["software", "no_offloading", "batch", "fusion"]
d_vec = [3, 5, 7, 9, 11, 13, 15]
p_str = "0.001"


def read_data(folder: str) -> list[float]:
    latency = []
    for d in d_vec:
        with open(os.path.join(folder, f"d_{d}.txt"), "r", encoding="utf8") as f:
            for line in f.readlines():
                line = line.strip("\r\n ")
                if line == "" or line.startswith("#"):
                    continue
                spt = line.split(" ")
                assert len(spt) == 3
                if spt[0] == p_str:
                    latency.append(float(spt[1]))
    return latency


for name in names:
    latency = read_data(os.path.join(this_dir, f"circuit_level_{name}"))
    plt.loglog(d_vec, latency, "o-", label=f"{name}")

plt.xlabel("code distance $d$")
plt.ylabel("decoding latency")
plt.legend()
plt.savefig(os.path.join(this_dir, f"circuit_level_compare.pdf"))
