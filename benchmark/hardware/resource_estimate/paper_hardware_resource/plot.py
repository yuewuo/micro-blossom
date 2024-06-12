import matplotlib.pyplot as plt
from dataclasses import dataclass, field

names = {
    "code_capacity": "code capacity",
    "phenomenological": "phenomenological",
    "circuit": "circuit-level (basic)",
    # "circuit_offload": "circuit-level (pre)",
    "circuit_fusion": "circuit-level (full)",
    "circuit_fusion_context_1024": "circuit-level (1k context)",
}

LUT_TOTAL = 899840
REG_TOTAL = 1799680

LUT_MIN = 1e3
VN_MIN = 3
VN_MAX = 5000


@dataclass
class Curve:
    display: str
    vertex_num_vec: list[int] = field(default_factory=lambda: [])
    edge_num_vec: list[int] = field(default_factory=lambda: [])
    offloading_num_vec: list[int] = field(default_factory=lambda: [])
    d_vec: list[int] = field(default_factory=lambda: [])
    lut_vec: list[int] = field(default_factory=lambda: [])
    reg_vec: list[int] = field(default_factory=lambda: [])


curves = []
for name, display in names.items():
    curve = Curve(display=display)
    with open(f"resource_{name}.txt", "r", encoding="utf-8") as f:
        for line in f.readlines():
            line = line.strip("\r\n ")
            if line.startswith("#") or line == "":
                continue
            values = line.split(" ")
            d = int(values[0])
            lut = int(values[1])
            assert abs(float(values[2]) - (lut / LUT_TOTAL * 100)) < 0.02
            reg = int(values[3])
            assert abs(float(values[4]) - (reg / REG_TOTAL * 100)) < 0.02
            vertex_num = int(values[5])
            edge_num = int(values[6])
            offloading_num = int(values[7])
            curve.d_vec.append(d)
            curve.lut_vec.append(lut)
            curve.reg_vec.append(reg)
            curve.vertex_num_vec.append(vertex_num)
            curve.edge_num_vec.append(edge_num)
            curve.offloading_num_vec.append(offloading_num)
    curves.append(curve)


def plot_lut():
    plt.cla()
    for curve in curves:
        plt.loglog(curve.vertex_num_vec, curve.lut_vec, "o-", label=curve.display)
    plt.loglog(
        [VN_MIN, VN_MAX],
        [LUT_TOTAL, LUT_TOTAL],
        ":",
        color="grey",
        label="available in VMK180",
    )
    plt.xlabel("Number of Vertices $|V|$")
    plt.xlim(VN_MIN, VN_MAX)
    plt.ylabel("Number of CLB LUTs")
    plt.ylim(LUT_MIN, LUT_TOTAL * 2)
    plt.legend()
    plt.savefig("lut.pdf")


plot_lut()
