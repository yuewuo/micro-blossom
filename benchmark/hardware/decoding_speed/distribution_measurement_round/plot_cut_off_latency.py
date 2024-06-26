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
logical_error_rate_of = lambda nm: 4.075622523830063e-06  # * nm / 9


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
        return original_latency.bias_latency(self.additional_latency)


nm_vec = list(range(3, 20))
figure_configs = [
    FigureConfig(prefix="fusion"),
    FigureConfig(prefix="batch"),
]

this_dir = os.path.dirname(os.path.abspath(__file__))


plt.cla()
fig, ax = plt.subplots()

for idx, init_config in enumerate(figure_configs):
    cut_off_latency_vec = []
    average_latency = []
    # latency_err = []
    for nm_idx, nm in enumerate(nm_vec):
        keys = init_config.__dict__
        keys["nm"] = nm
        config = FigureConfig(**keys)
        distribution = config.get_distribution()
        filtered_distribution = distribution.filter_latency_range(
            config.min_latency, config.max_latency
        )
        combined_distribution = filtered_distribution.combine_bins(
            combine_bin=BIN_COMBINE
        )
        x_vec, y_vec = combined_distribution.flatten()

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

        # fit the tail
        counts_records = combined_distribution.count_records()
        min_freq = 1e-6
        max_freq = 1e-4
        A, B = combined_distribution.fit_exponential_tail(f_range=(min_freq, max_freq))
        # print(f"freq = exp({A} - {B} * latency)")

        # use the fit curve to calculate the cut-off latency
        # accumulated probability is exp(A - B*L) / B = probability
        probability = logical_error_rate_of(nm)
        fit_cut_off_latency = (A - np.log(probability * B)) / B
        cut_off_latency_vec.append(fit_cut_off_latency)

        # use real data
        # cut_off_latency = distribution.find_cut_off_latency(logical_error_rate_of(nm))
        # cut_off_latency_vec.append(cut_off_latency)
        # latency_err.append(cut_off_latency * distribution.interval_ratio)

        average_latency.append(distribution.average_latency())

    ax.plot(
        nm_vec,
        cut_off_latency_vec,
        "o-",
        label=init_config.prefix + " cut-off",
    )

    ax.plot(
        nm_vec,
        average_latency,
        "o-",
        label=init_config.prefix + " average",
    )

plt.yscale("log")
plt.ylabel("cut-off latency")
plt.xlabel("measurement rounds")
plt.legend()
plt.savefig(f"cut_off_latency.pdf")
