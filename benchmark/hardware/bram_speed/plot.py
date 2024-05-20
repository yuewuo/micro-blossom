import matplotlib.pyplot as plt

names = ["write", "read", "w-r", "r-w", "r128", "r256"]

fs = []
ys = {name: [] for name in names}

with open("data.txt", "r", encoding="utf8") as f:
    for line in f.readlines():
        line = line.strip("\r\n ")
        if line.startswith("#") or line == "":
            continue
        values = [float(e) for e in line.split(" ")]
        assert len(values) == len(names) + 1
        fs.append(values[0])
        for idx, value in enumerate(values[1:]):
            ys[names[idx]].append(value)

clock_cycles = [1e3 / f for f in fs]

for name in names:
    plt.plot(clock_cycles, ys[name], "o-", label=name)

plt.xlim(0, 10.5)
plt.xlabel("clock cycle (ns)")
plt.ylabel("latency (ns)")
plt.title("Memory Operation Latency")
plt.legend()
plt.savefig("bram_speed.pdf")
