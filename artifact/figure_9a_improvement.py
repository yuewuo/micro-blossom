import git
import sys
import os
import math
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
    label: str
    folder: str  # relative to the `speed_dir`

    @property
    def abs_folder(self):
        return os.path.join(speed_dir, self.folder)

    def run(self):
        run_file = os.path.join(self.abs_folder, "run.py")
        print(f"running: `python3 {run_file}`")
        subprocess.run(["python3", run_file])


figure_configs = [
    FigureConfig(label="Parity Blossom (CPU)", folder="circuit_level_software"),
    FigureConfig(
        label="$+\,$parallel dual phase", folder="circuit_level_no_offloading"
    ),
    FigureConfig(label="$+\,$parallel primal phase", folder="circuit_level_batch"),
    FigureConfig(label="$+\,$round-wise fusion", folder="circuit_level_fusion"),
]


def main():
    # you can modify this to any of the values among:
    # ['3.981071705534972e-05', '6.309573444801933e-05', '0.0001', '0.00015848931924611136', '0.000251188643150958',
    # '0.00039810717055349724', '0.0006309573444801934', '0.001', '0.0015848931924611134', '0.0025118864315095794',
    # '0.003981071705534973', '0.006309573444801933', '0.01']
    p_str = "0.001"
    # draw the arrow for the comparison at this code distance
    d_compare = 13

    for config in figure_configs:
        title_print(f"running in folder {config.abs_folder}")
        time.sleep(1)
        config.run()

        p_vec = available_p_vec(figure_configs[0])
        assert (
            p_str in p_vec
        ), f"p_str = {p_str} is not available; choose one from {p_vec}"

    title_print("plotting the comparison figure")

    fig, ax = plt.subplots()
    ax2 = ax.twinx()
    ax2.set_ylim(math.log(1e-7), math.log(3e-5))

    for idx, config in enumerate(figure_configs):
        latency = latency_of(config, p_str)
        style = ["o-", "^-", "s-", "p-"]
        ax.loglog(d_vec, latency, style[idx], label=f"{config.label}")

    # draw improvement
    d_idx = d_vec.index(d_compare)
    delta = 0.13
    for idx in range(3):
        config1 = figure_configs[idx]
        config2 = figure_configs[idx + 1]
        latency1 = latency_of(config1, p_str)[d_idx]
        latency2 = latency_of(config2, p_str)[d_idx]
        for l1, l2, de in [(latency1, latency2, delta), (latency2, latency1, -delta)]:
            ax2.arrow(
                d_compare,
                math.log(l1) - de,
                0,
                math.log(l2) - math.log(l1) + 2 * de,
                head_width=0.2,
                head_length=0.04,
                linewidth=1.5,
                color="grey",
                length_includes_head=False,
            )
        ratio = latency1 / latency2
        ax2.text(
            d_compare - 2.8,
            math.log(latency1) - 0.4,
            f"{ratio:.1f}$\\times$",
            size=18,
        )

    ax.set_xlim(2.1, 16)
    x_ticks = list(range(3, 16))
    ax.set_xticks(x_ticks, [str(d) if d in d_vec else "" for d in x_ticks])
    ax.set_xlabel("code distance $d$")

    ax.set_ylim(1e-7, 3e-5)
    ax.set_yticks([1e-7, 1e-6, 1e-5], ["0.1", "1", "10"])
    ax.set_ylabel("decoding latency $L$ ($\mu s$)")

    ax2.tick_params(left=False, right=False, labelright=False)

    ax.legend()
    plt.title(f"Decoding latency improvement at $p = {p_str}$")
    plt.savefig(os.path.join(this_dir, f"figure_9a.pdf"))


def title_print(*args, **kwargs):
    print("\n\n####################################################")
    print(*args, **kwargs)
    print("####################################################\n\n")


d_vec = [3, 5, 7, 9, 11, 13, 15]


def available_p_vec(config: FigureConfig) -> list[str]:
    with open(
        os.path.join(config.abs_folder, f"d_{d_vec[0]}.txt"), "r", encoding="utf8"
    ) as f:
        p_str_vec = []
        for line in f.readlines():
            line = line.strip("\r\n ")
            if line == "" or line.startswith("#"):
                continue
            spt = line.split(" ")
            assert len(spt) == 3
            p_str_vec.append(spt[0])
    return p_str_vec


def latency_of(config: FigureConfig, p_str: str) -> list[float]:
    latency = []
    for d in d_vec:
        with open(
            os.path.join(config.abs_folder, f"d_{d}.txt"), "r", encoding="utf8"
        ) as f:
            for line in f.readlines():
                line = line.strip("\r\n ")
                if line == "" or line.startswith("#"):
                    continue
                spt = line.split(" ")
                assert len(spt) == 3
                if spt[0] == p_str:
                    latency.append(float(spt[1]))
    return latency


if __name__ == "__main__":
    main()
