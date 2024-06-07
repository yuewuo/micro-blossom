import matplotlib.pyplot as plt
from dataclasses import dataclass, field

names = {
    "code_capacity": "code capacity",
    "phenomenological": "phenomenological",
    "circuit": "circuit-level",
    "circuit_offload": "circuit-level (pre)",
    "circuit_fusion": "circuit-level (pre+fusion)",
    "circuit_fusion_context_1024": "circuit-level (pre+fusion+1024 context)",
}

LUT_TOTAL = 899840
REG_TOTAL = 1799680

LUT_MIN = 500
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
    fig, ax1 = plt.subplots()
    for curve in curves:
        ax1.loglog(curve.vertex_num_vec, curve.lut_vec, "o-", label=curve.display)
    ax2 = ax1.twinx()
    plt.xlabel("Number of Vertices $|V|$")
    plt.xlim(VN_MIN, VN_MAX)
    ax1.set_ylabel("Number of CLB LUTs")
    ax1.set_ylim(LUT_MIN, LUT_TOTAL)
    ax2.set_ylabel("Percentage")
    ax2.set_ylim([LUT_MIN / LUT_TOTAL, 1])

    ax2.set_yticks(
        [i / 10 for i in range(11)],
        [f"{i}0%" for i in range(11)],
    )
    ax1.legend()
    plt.savefig("lut.pdf")


def plot_registers():
    plt.cla()
    for curve in curves:
        plt.plot(curve.vertex_num_vec, curve.reg_vec)
    plt.xlabel("Number of Vertices $|V|$")
    plt.savefig("registers.pdf")


plot_lut()
plot_registers()
