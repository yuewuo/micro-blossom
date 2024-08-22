from dataclasses import dataclass
import matplotlib.pyplot as plt
import os

this_dir = os.path.dirname(os.path.abspath(__file__))


@dataclass
class Config:
    name: str
    color: str
    uplink_bandwidths: list[float] | None = None
    downlink_bandwidths: list[float] | None = None


p_per_10 = 5
p_vec = [1e-4 * (10 ** (i / p_per_10)) for i in range(-1, p_per_10 * 2 + 1)]


def read_file_column_float(filepath: str, column_index: int) -> list[float]:
    result = []
    with open(filepath, "r", encoding="utf8") as f:
        idx = 0
        for line in f.readlines():
            line = line.strip("\r\n ")
            if line.startswith("#"):
                continue
            spt = line.split(" ")
            assert abs(float(spt[0]) - p_vec[idx]) / p_vec[idx] <= 1e-3, "mismatch p"
            idx += 1
            result.append(float(spt[column_index]))
    return result


def read_bandwidth(filepath: str, column_index: int) -> list[float]:
    d = 9
    # in Mbps
    return [e / d for e in read_file_column_float(filepath, column_index)]


COLUMN_INDEX_OUT_BITS = 7
COLUMN_INDEX_IN_BITS = 9
COLUMN_INDEX_NAIVE_OUT_BITS = 10
COLUMN_INDEX_NAIVE_IN_BITS = 11
COLUMN_INDEX_COMPRESSED_IN_BITS = 12
COLUMN_INDEX_COMPRESSED_OUT_BITS = 12


configs = [
    Config(
        name="naive",
        uplink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "no_offloading_d9.txt"), COLUMN_INDEX_NAIVE_IN_BITS
        ),
        downlink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "no_offloading_d9.txt"), COLUMN_INDEX_NAIVE_OUT_BITS
        ),
        color="tab:blue",
    ),
    Config(
        name="compressed",
        uplink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "no_offloading_d9.txt"),
            COLUMN_INDEX_COMPRESSED_IN_BITS,
        ),
        downlink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "no_offloading_d9.txt"),
            COLUMN_INDEX_COMPRESSED_OUT_BITS,
        ),
        color="tab:orange",
    ),
    Config(
        name="+ parallel dual",
        uplink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "no_offloading_d9.txt"), COLUMN_INDEX_IN_BITS
        ),
        downlink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "no_offloading_d9.txt"), COLUMN_INDEX_OUT_BITS
        ),
        color="tab:green",
    ),
    Config(
        name="+ pre-matching",
        uplink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "pre_match_d9.txt"), COLUMN_INDEX_IN_BITS
        ),
        downlink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "pre_match_d9.txt"), COLUMN_INDEX_OUT_BITS
        ),
        color="tab:red",
    ),
    Config(
        name="+ fusion",
        uplink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "layer_fusion_d9.txt"), COLUMN_INDEX_IN_BITS
        ),
        downlink_bandwidths=read_bandwidth(
            os.path.join(this_dir, "layer_fusion_d9.txt"), COLUMN_INDEX_OUT_BITS
        ),
        color="tab:purple",
    ),
]

print(configs)

for link in ["uplink", "downlink"]:
    plt.cla()

    for config in configs:
        bandwidths = None
        if link == "uplink":
            bandwidths = config.uplink_bandwidths
        else:
            bandwidths = config.downlink_bandwidths
        if bandwidths is None:
            continue

        plt.loglog(p_vec, bandwidths, label=config.name, color=config.color)

        plt.xlabel("physical error rate $p$")
        plt.ylabel("bandwidth ($Mbps$)")
        plt.title(f"{link} bandwidth")
        plt.ylim(0.08, 2e3)
        plt.legend()
        plt.savefig(os.path.join(this_dir, f"{link}.pdf"))
