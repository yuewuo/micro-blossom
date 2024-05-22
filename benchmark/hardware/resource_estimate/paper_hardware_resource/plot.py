import matplotlib.pyplot as plt
from dataclasses import dataclass

names = {
    "code_capacity": "code capacity",
    # "phenomenological": "phenomenological",
    # "circuit": "circuit-level",
    # "circuit_offload": "circuit-level (pre)",
    # "circuit_fusion": "circuit-level (pre+fusion)",
}

LUT_TOTAL = 899840
REG_TOTAL = 1799680


@dataclass
class Curve:
    display: str
    d_vec: list[int]
    lut_vec: list[int]
    reg_vec: list[int]


curves = []
for name, display in names.items():
    curve = Curve(display=display, d_vec=[], lut_vec=[], reg_vec=[])
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
            curve.d_vec.append(d)
            curve.lut_vec.append(lut)
            curve.reg_vec.append(reg)
    curves.append(curve)


def plot_lut():
    plt.cla()
    for curve in curves:
        plt.plot(curve.d_vec, curve.lut_vec)
    plt.xlabel("code distance $d$")
    plt.savefig("lut.pdf")


def plot_registers():
    plt.cla()
    for curve in curves:
        plt.plot(curve.d_vec, curve.lut_vec)
    plt.xlabel("code distance $d$")
    plt.savefig("registers.pdf")


plot_lut()
plot_registers()
