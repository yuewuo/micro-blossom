import git
import sys
import os
import time
import subprocess
from dataclasses import dataclass
import matplotlib.pyplot as plt

this_dir = os.path.dirname(os.path.abspath(__file__))
git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
speed_dir = os.path.join(git_root_dir, "benchmark", "hardware", "decoding_speed")
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "benchmark", "slurm_utilities"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))


@dataclass
class FigureConfig:
    name: str
    folder: str  # relative to the `speed_dir`

    @property
    def abs_folder(self):
        return os.path.join(speed_dir, self.folder)

    def run(self):
        run_file = os.path.join(self.abs_folder, "run.py")
        print(f"running: `python3 {run_file}`")
        subprocess.run(["python3", run_file])


figure_configs = [
    FigureConfig(name="section_5", folder="circuit_level_no_offloading"),
    FigureConfig(name="section_6", folder="circuit_level_batch"),
    FigureConfig(name="section_7", folder="circuit_level_fusion"),
    FigureConfig(name="software", folder="circuit_level_software"),
]


def main():
    for config in figure_configs:
        title_print(f"running config: {config.name} in folder {config.abs_folder}")
        time.sleep(1)
        config.run()

        title_print(f"plotting config: {config.name} in folder {config.abs_folder}")
        plot(config, f"figure_8_{config.name}.pdf")


def title_print(*args, **kwargs):
    print("\n\n####################################################")
    print(*args, **kwargs)
    print("####################################################\n\n")


def plot(config: FigureConfig, filename: str):

    d_vec = [3, 5, 7, 9, 11, 13, 15]
    p_per_10 = 5
    p_vec = [1e-4 * (10 ** (i / p_per_10)) for i in range(-2, p_per_10 * 2 + 1)]

    plt.cla()
    fig, ax = plt.subplots()
    # draw dashed lines
    for y in [1e-6, 1e-5, 1e-4]:
        ax.plot(
            [min(p_vec) / 1.3, max(p_vec) * 1.3],
            [y, y],
            linestyle="dotted",
            linewidth=0.8,
            color="lightgrey",
        )

    for i, d in enumerate(d_vec):
        with open(
            os.path.join(config.abs_folder, f"d_{d}.txt"), "r", encoding="utf8"
        ) as f:
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
        style = ["o-", "^-", "s-", "p-", "*-", "+-", "x-"]
        ax.loglog(p_data, average_latency_data, style[i], label=f"$d = {d}$")
    ax.set_ylim(1e-7, 1e-3)
    y_ticks = [1e-7, 1e-6, 1e-5, 1e-4, 1e-3]
    ax.set_yticks(y_ticks, ["0.1", "1", "10", "100", "1000"])
    ax.set_ylabel(r"decoding latency $L$ ($\mu s$)")
    ax.set_xlim(min(p_vec) / 1.3, max(p_vec) * 1.3)
    ax.set_xticks([1e-4, 1e-3, 1e-2], [r"$0.01\%$", r"$0.1\%$", r"$1\%$"])
    ax.set_xlabel("physical error rate $p$")
    ax.legend(reverse=True)
    plt.title(f"Decoding Latency of {config.name}")
    plt.savefig(os.path.join(this_dir, filename))


if __name__ == "__main__":
    main()
