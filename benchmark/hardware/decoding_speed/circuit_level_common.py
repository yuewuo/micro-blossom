import os
import matplotlib.pyplot as plt

p_per_10 = 5
p_vec = [1e-4 * (10 ** (i / p_per_10)) for i in range(-2, p_per_10 * 2 + 1)]
d_vec = [3, 5, 7, 9, 11, 13, 15]


def save_data(data, this_dir: str):
    for d, latency_vec in zip(d_vec, data):
        with open(os.path.join(this_dir, f"d_{d}.txt"), "w", encoding="utf8") as f:
            f.write("# <p> <average latency> <samples>\n")
            for p, latency in zip(p_vec, latency_vec):
                f.write(f"{p} {latency.average_latency()} {latency.count_records()}\n")


def plot_data(this_dir: str):
    name = os.path.basename(this_dir)
    plt.cla()
    for d in d_vec:
        with open(os.path.join(this_dir, f"d_{d}.txt"), "r", encoding="utf8") as f:
            p_data = []
            average_latency_data = []
            for line in f.readlines():
                line = line.strip("\r\n ")
                if line == "" or line.startswith("#"):
                    continue
                spt = line.split(" ")
                assert len(spt) == 3
                p_data.append(float(spt[0]))
                average_latency_data.append(float(spt[1]))
        plt.loglog(p_data, average_latency_data, "o-", label=f"$d = {d}$")
    plt.savefig(os.path.join(this_dir, f"{name}.pdf"))
