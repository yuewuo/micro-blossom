import os, sys, git, math
from dataclasses import dataclass
import matplotlib.pyplot as plt
import matplotlib as mpl
import numpy as np


git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from main_benchmark_decoding import *


# combine multiple bins together
BIN_COMBINE = 10

# d=9, p=0.001
logical_error_rate = 4.075622523830063e-06


@dataclass
class FigureConfig:
    prefix: str
    nm: int = None
    additional_latency: float = 0
    min_latency: float = 1e-7
    max_latency: float = 1e-4

    @property
    def name(self):
        return f"{self.prefix}_{self.nm}"

    def get_distribution(self) -> TimeDistribution:
        filename = os.path.join(this_dir, f"{self.name}.txt")
        assert os.path.exists(filename)
        with open(filename, "r", encoding="utf8") as f:
            original_latency = TimeDistribution.from_line(f.read())
        distribution = TimeDistribution()
        for latency, count in zip(*original_latency.flatten()):
            latency += self.additional_latency
            distribution.record(latency, count)
        return distribution

    def flatten_latency_filtered(self, distribution: TimeDistribution):
        x_vec = []
        y_vec = []
        for latency, count in zip(*distribution.flatten()):
            if latency < self.min_latency or latency > self.max_latency:
                assert (
                    count <= 1
                ), f"[warning] {self.name} latency {latency} has count {count}"
                continue
            x_vec.append(latency)
            y_vec.append(count)

        cx_vec = []
        cy_vec = []
        for idx in range(len(x_vec) // BIN_COMBINE):
            start = idx * BIN_COMBINE
            end = (idx + 1) * BIN_COMBINE
            x = sum(x_vec[start:end]) / BIN_COMBINE
            y = sum(y_vec[start:end])
            cx_vec.append(x)
            cy_vec.append(y)
        return cx_vec, cy_vec

    def find_cut_off_latency(
        self, distribution: TimeDistribution, multi_pL: float = 1
    ) -> float:
        cut_off_pL = logical_error_rate * multi_pL
        cut_off_count = distribution.count_records() * cut_off_pL
        assert cut_off_count >= 10, "otherwise not accurate enough"
        # accumulate from right most
        x_vec, y_vec = distribution.flatten()
        accumulated = 0
        for idx in reversed(range(0, len(x_vec))):
            accumulated += y_vec[idx]
            if accumulated >= cut_off_count:
                return x_vec[idx + 1]


nm_vec = [2, 3, 4, 5, 6, 7, 8, 9, 19]
figure_configs = [
    FigureConfig(prefix="fusion"),
    FigureConfig(prefix="batch"),
]

colors = [
    "tab:blue",
    "tab:orange",
    "tab:green",
    "tab:red",
    "tab:purple",
    "tab:brown",
    "tab:pink",
    "tab:gray",
    "tab:olive",
    "tab:cyan",
]
this_dir = os.path.dirname(os.path.abspath(__file__))


for idx, init_config in enumerate(figure_configs):
    plt.cla()
    fig, ax = plt.subplots()
    for nm_idx, nm in enumerate(nm_vec):
        keys = init_config.__dict__
        keys["nm"] = nm
        config = FigureConfig(**keys)
        distribution = config.get_distribution()
        x_vec, y_vec = config.flatten_latency_filtered(distribution)
        # we are plotting log y axis
        y_resolution = 100
        y_to_count = lambda samples: (
            int(math.log(samples) * y_resolution) if samples != 0 else 0
        )
        count_to_y = lambda count: (
            int(math.exp(count / y_resolution)) if count != 0 else 0
        )
        latency_to_x = (
            lambda latency: math.log(latency / x_vec[0])
            / math.log(x_vec[-1] / x_vec[0])
            * len(x_vec)
            + 0.5
        )
        x_to_latency = lambda i: min(x_vec) * (
            (max(x_vec) / min(x_vec)) ** ((i - 0.5) / (len(x_vec) - 1))
        )

        # construct histogram data
        # we need to manually generate the data for histogram...
        # since it is a log-log plot
        data = np.array([])
        cut_off_latency = config.find_cut_off_latency(distribution)
        cutoff_data = np.array([])
        cutoff_idx = None
        for i, y in enumerate(y_vec):
            count = y_to_count(y)
            data = np.append(data, np.repeat(i, count))
            if x_vec[i] >= cut_off_latency:
                cutoff_data = np.append(cutoff_data, np.repeat(i, count))
                if cutoff_idx is None:
                    cutoff_idx = i
        ax.hist(
            data,
            range=(0, len(x_vec)),
            bins=len(x_vec),
            histtype="stepfilled",
            facecolor=(1, 1, 1, 0),
            edgecolor=colors[nm_idx],
            linewidth=2,
            # alpha=0.5,
            label=f"nm = {nm}",
        )

    # x ticks
    xticks_values = []
    xticks_labels = []
    for label, value in [
        ("$0.1$", 1e-7),
        ("$1$", 1e-6),
        ("$10$", 1e-5),
    ]:
        xticks_values.append(latency_to_x(value))
        xticks_labels.append(label)
        if value != 1e-2:
            for i in range(2, 10):
                xticks_values.append(latency_to_x(value * i))
                xticks_labels.append("")
    ax.set_xlim(-5, len(x_vec) + 5)
    ax.set_xticks(xticks_values, xticks_labels)
    # y ticks
    yticks_values = []
    yticks_labels = []
    for label, scale in [
        ("$1$", 1),
    ] + [(f"$10^{{-{idx}}}$", 10 ** (-idx)) for idx in range(1, 9)]:
        value = scale * distribution.count_records()
        yticks_values.append(y_to_count(value))
        yticks_labels.append(label)
        # draw dashed lines
        ax.plot(
            [-5, len(x_vec) + 5],
            [y_to_count(value), y_to_count(value)],
            linestyle="dotted",
            linewidth=0.8,
            color="lightgrey",
            zorder=-10,
        )

    plt.yticks(yticks_values, yticks_labels)
    plt.ylim(0, y_to_count(3e7))
    plt.ylabel("probability $P(L)$")
    plt.xlabel("decoding latency ($\mu s$)")
    plt.legend()
    plt.savefig(f"{config.prefix}2.pdf")
